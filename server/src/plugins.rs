// SPDX-License-Identifier: GPL-3.0-or-later

use crate::MyError;
use std::{
    fmt, io,
    path::PathBuf,
    sync::{Mutex, OnceLock},
};
use tracing::debug;
use wasmtime::{
    Config, Engine, ExternType, Instance, Linker, Memory, MemoryTypeBuilder, Module, Store,
    StoreLimits, StoreLimitsBuilder,
};
use wasmtime_wasi::{
    WasiCtxBuilder,
    p1::{self, WasiP1Ctx},
};

/// Default maximum number of WASM [Instance]s our [Store] will hold.
const DEFAULT_STORE_MAX_INSTANCE_COUNT: usize = 2;
/// Default maximum number of linear memory pages our [Store] will allow.
const DEFAULT_STORE_MAX_PAGE_COUNT: usize = 33;
/// Maximum allowed byte length of `data` argument in a hash call.
const MAX_PAYLOAD_LEN_BYTES: usize = 1024;
// const SERVER_DIR: &'static str = env!("CARGO_MANIFEST_DIR");
const SERVER_DIR: &str = env!("CARGO_MANIFEST_DIR");

/// A WASM [Module] that implements our hashing _Interface_.
// #[derive(Debug)]
struct Plugin {
    /// The Plugin ID (aka short name; e.g. `fx`) we use to construct a full file object name
    /// expected to be the WASM binary located in our 'plugins' folder.
    id: String,
    /// A single [Instance] of the WASM [Module] we'll use.
    instance: Instance,
}

impl fmt::Debug for Plugin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Plugin{{id: {}, instance: {:?}, ...}}",
            self.id, self.instance
        )
    }
}

/// NOTE (rsn) 20260803 - i'm trying to control the allocation of memory by introducing a [Store]
/// resource-limiter.
/// NOTE (rsn) 20260805 - a 1 Page / Plugin is NOT working :( Instantiating a WASI P1 Module
/// requires a minimum of 16 to 17 Pages of 64KB memory each (~1MB) as discovered lately :(
/// fortunately i can amend the limiter at runtime!  for now pre-set 2 alternatives.
/// NOTE (rsn) 20260807 - changing the limiter at runtime works, but does NOT help finding out what
/// is the available/used Store memory --at least i couldnt find an API call that gives me those
/// answers.  for now, stick w/ 1 limiter and a hard-wired limit that satisfies the requirements of
/// the 2 plugins of interest.
struct MyState {
    state: WasiP1Ctx,
    limits: StoreLimits,
}

static PLUGIN_MGR: OnceLock<Mutex<PluginMgr>> = OnceLock::new();
pub(crate) fn plugin_mgr() -> &'static Mutex<PluginMgr> {
    PLUGIN_MGR.get_or_init(|| {
        let mut it = PluginMgr::default();
        it.load("fx").expect("Failed :(");
        it.load("xx").expect("Failed :(");
        Mutex::new(it)
    })
}

/// A glorified name for a structure grouping objects representing WASM core concept elements.
///
/// Documentation of individual fields are lifted from
/// <https://docs.rs/wasmtime/46.0.1/wasmtime/index.html#core-concepts>.
pub struct PluginMgr {
    /// [Engine] - a global compilation and runtime environment for _WebAssembly_. An [Engine] is
    /// an object that can be shared concurrently across threads and is created with a Config with
    /// many knobs for configuring behavior. Compiling or executing any _WebAssembly_ requires first
    /// configuring and creating an [Engine]. All _Modules_ and _Components_ belong to an [Engine],
    /// and typically there’s one [Engine] per process.
    engine: Engine,

    /// [Linker] - host functions are defined within a linker to provide them a string-based name
    /// which can be looked up when instantiating a WebAssembly module or component. Linkers are
    /// traditionally populated at startup and then reused for all future instantiations of all
    /// instances, assuming the set of host functions does not change over time. Host functions are
    /// `Fn(..) + Send + Sync`` and typically do not close over mutable state. Instead it’s
    /// recommended to store mutable state in the `T`` of `Store<T>` which is accessed through
    /// `Caller<'_, T>`` provided to host functions.
    linker: Linker<MyState>,

    /// [Store] - container for all information related to WebAssembly objects such as functions,
    /// instances, memories, etc. A `Store<T>` allows customization of the T to store arbitrary host
    /// data within a [Store]. This host data can be accessed through host functions via the Caller
    /// function parameter in host-defined functions. A [Store] is required for all WebAssembly
    /// operations, such as calling a wasm function. The [Store] is passed in as a “context” to
    /// methods like Func::call.
    /// Dropping a [Store] will deallocate all memory associated with WebAssembly objects within the
    /// [Store]. A [Store] is cheap to create and destroy and does not GC objects such as unused
    /// instances internally, so it’s intended to be short-lived (or no longer than the instances it
    /// contains).
    store: Store<MyState>,

    /// collection of loaded + ready Plugin instances.
    plugins: Vec<Plugin>,
}

impl Default for PluginMgr {
    fn default() -> Self {
        Self::try_from(()).expect("Failed initializing PluginManager :(")
    }
}

impl TryFrom<()> for PluginMgr {
    type Error = MyError;

    fn try_from(_: ()) -> Result<Self, Self::Error> {
        // NOTE (rsn) 20260807 - using either a custom Config to enable custom page size when
        // instantiating an Engine, or simply using the default, and later instantiating a Store w/
        // a custom MemoryType using a 1KB value as the page size yields the same error saying that
        // ONLY 64KB or 1-byte page sizes are allowed :(
        let engine = Engine::new(Config::new().wasm_custom_page_sizes(true))
            .expect("Failed configuring Engine w/ custom memory page size :(");
        let wasip1 = WasiCtxBuilder::new()
            .inherit_stdio()
            .inherit_env()
            .build_p1();

        // NOTE (rsn) 20260806 - WASM binaries are created and packaged elsewhere.  what their
        // minimum memory page requirement is unknown, and to my knowledge is not part of WASI
        // specs.  as a consequence setting a hard limit for the store here, at creation time, may
        // cause problems later. For example i used to set a hard limit of 2 pages for the Store but
        // discovered that the 2 plugins i'm using require 16 (fx) and 17 pages (xx) :(
        // NOTE (rsn) 20260807 - even though i found out one can modify a Store "limiter" _after_
        // creating the Store, that doesn't help reducing memory requirements.  for now stick w/ a
        // hard total of 33 pages each being 64KB large.
        debug!(
            "Will limit Store memory to {} pages",
            DEFAULT_STORE_MAX_PAGE_COUNT
        );
        let my_state = MyState {
            state: wasip1,
            limits: StoreLimitsBuilder::new()
                .instances(DEFAULT_STORE_MAX_INSTANCE_COUNT)
                .memory_size(DEFAULT_STORE_MAX_PAGE_COUNT * 64 * 1024) // in bytes...
                .build(),
        };
        debug!("Store limits = {:?}", my_state.limits);
        let mut store = Store::new(&engine, my_state);
        store.limiter(|state| &mut state.limits);
        let mut linker = Linker::new(&engine);
        p1::add_to_linker_sync(&mut linker, |state: &mut MyState| &mut state.state)
            .expect("Failed adding synchronous version of WASI P1 functions to the Linker :(");
        // limit the store's memory... see
        // https://docs.rs/wasmtime/47.0.3/wasmtime/struct.MemoryTypeBuilder.html
        // IMPORTANT (rsn) 20260803 - page size can only be set to 1 byte (too low), or 64 kilo-bytes
        // (too high).  ideally page-size for our plugins should be MAX_PAYLOAD_LENGTH (1024 bytes),
        // w/ 1 page worth of bytes per instance...
        let memory_type = MemoryTypeBuilder::new()
            // .page_size_log2(10) // 1024 bytes -  DOESN'T WORK :(
            .max(Some(DEFAULT_STORE_MAX_PAGE_COUNT.try_into().expect(
                "Failed converting Store max memory page count: usize -> u64",
            )))
            .build()
            .expect("Failed configuring MemoryType :(");
        Memory::new(&mut store, memory_type)
            .expect("Failed creating + assigning Memory to Store :(");
        debug!("Created + assigned Memory to Store!");
        Ok(Self {
            engine,
            linker,
            store,
            plugins: Vec::new(),
        })
    }
}

impl PluginMgr {
    /// Load, and compile a Plugin's [Module] given its ID, then instantiate
    /// and cache one single [Instance].
    pub fn load(&mut self, mid: &str) -> Result<(), MyError> {
        if self.plugins.iter().find(|p| p.id == mid).is_some() {
            debug!("Plugin '{}' is already loaded. Do nothing", mid);
            return Ok(());
        }

        // check if we still have room...
        if self.plugins.len() == DEFAULT_STORE_MAX_INSTANCE_COUNT {
            return Err(MyError::Runtime(
                format!("Store is full. No room for Plugin '{}' :(", mid).into(),
            ));
        }

        debug!("About to load Plugin '{}'...", mid);
        let plugin_wasm_file = plugin_loc(mid)?;
        let module = Module::from_file(&self.engine, plugin_wasm_file)?;

        // ensure it exports 'memory' as a WASI P1 component should...
        let memory = module.get_export("memory").ok_or(MyError::Runtime(
            format!("'memory' export was NOT found in Plugin '{}' :(", mid).into(),
        ))?;

        match memory {
            ExternType::Memory(memory_type) => {
                if memory_type.is_shared() {
                    return Err(MyError::Runtime(
                        format!("Plugin '{}' exports its memory as shared :(", mid).into(),
                    ));
                }

                let mod_min_mem = memory_type.minimum();
                debug!(
                    "Plugin '{}' (export) memory requires at least {} page(s)",
                    mid, mod_min_mem
                );

                let store_max_page_count_u64: u64 = DEFAULT_STORE_MAX_PAGE_COUNT
                    .try_into()
                    .expect("Failed converting Store max memory: usize -> u64");
                assert!(
                    mod_min_mem <= store_max_page_count_u64,
                    "Plugin '{}' minimum memory page count ({}) exceeds our Store upper limit ({}) :(",
                    mid,
                    mod_min_mem,
                    store_max_page_count_u64
                );
            }
            _ => {
                return Err(MyError::Runtime(
                    format!("Plugin '{}' does NOT export its memory as expected :(", mid).into(),
                ));
            }
        };

        debug!("About to instantiate Plugin '{}'...", mid);
        let instance = self.linker.instantiate(&mut self.store, &module)?;

        let plugin = Plugin {
            id: mid.to_owned(),
            instance,
        };
        self.plugins.push(plugin);

        debug!("Plugin '{}' is ready!", mid);
        Ok(())
    }

    /// Fetch a Plugin [Instance] given its ID. Raise error if it's not already loaded.
    fn get(&mut self, mid: &str /* Plugin ID */) -> Result<Instance, MyError> {
        if let Some(p) = self.plugins.iter().find(|p| p.id == mid) {
            Ok(p.instance)
        } else {
            Err(MyError::Runtime(
                format!("Instance of Plugin '{}' was not found :(", mid).into(),
            ))
        }
    }

    /// Convenience method to invoke the single _Interface_ hash function of a
    /// plugin [Instance] w/ given parameters.
    ///
    /// When `salt` is `None`, we invoke the unsalted version.
    pub fn do_hash(
        &mut self,
        mid: &str, /* Plugin ID */
        seed: u32,
        data: &[u8],
    ) -> Result<u32, MyError> {
        let length = data.len();
        if length > MAX_PAYLOAD_LEN_BYTES {
            return Err(MyError::Runtime(
                format!(
                    "Data length ({}) exceeds maximum allowed limit ({}) :(",
                    length, MAX_PAYLOAD_LEN_BYTES
                )
                .into(),
            ));
        }

        let instance = self.get(mid)?;
        let memory = instance
            .get_memory(&mut self.store, "memory")
            .ok_or(MyError::Runtime(
                format!(
                    "'memory' export was NOT found in plugin '{}' instance :(",
                    mid
                )
                .into(),
            ))?;
        // NOTE (rsn) 20260804 - `wasmtime` documentation states..
        // "WebAssembly memories are made up of a whole number of pages, so the byte size returned
        // will always be a multiple of this memory's page size. Note that different Wasm memories
        // may have different page sizes. You can get a memory's page size via the Memory::page_size
        // method"
        let m_page_size = memory.page_size(&self.store);
        debug!(
            "Memory of plugin '{}' has a page size of {} bytes (or {} KB)",
            mid,
            m_page_size,
            m_page_size / 1024
        );
        let m_data_size_bytes = memory.data_size(&self.store);
        let m_data_size_bytes_u64: u64 = m_data_size_bytes
            .try_into()
            .expect("Failed converting Memory data size: usize -> u64");
        let m_data_pages = m_data_size_bytes_u64
            .checked_div(m_page_size)
            .expect("Unexpected None when dividing memory-data-size by page-size");
        let m_data_pages_rem = m_data_size_bytes_u64
            .checked_rem(m_page_size)
            .expect("Unexpected None when finding memory-data-size % page-size");
        assert_eq!(m_data_pages_rem, 0);
        debug!(
            "Linear memory of plugin '{}' is {} bytes (or {} pages)",
            mid, m_data_size_bytes, m_data_pages
        );
        // ensure it has enough space to accomodate provided data...
        if m_data_size_bytes < length {
            let msg = format!(
                "Linear memory ({} bytes) of plugin '{}' instance is too small :(",
                m_data_size_bytes, mid
            );
            return Err(MyError::Runtime(msg.into()));
        }

        // copy data bytes to linear memory...
        memory.data_mut(&mut self.store)[0..length].copy_from_slice(data);
        let func = instance.get_typed_func::<(u32, u32, u32), u32>(&mut self.store, "hash")?;
        let result = func.call(&mut self.store, (seed, 0, length as u32));
        // scrub used memory bytes before returning result...
        memory.data_mut(&mut self.store)[0..length].fill(0x00);
        result.map_err(MyError::Wasm)
    }
}

/// Return file system location of a plugin's WASM file given its ID.
///
/// Assume WASM files are named, based on their ID, as `<ID>_plugin.wasm` and are located in a
/// sub-folder named 'plugins' in the project folder.
fn plugin_loc(id: &str) -> Result<String, MyError> {
    let mut it = plugins_dir();
    it.push(format!("{}_plugin.wasm", id));
    if !it.exists() {
        return Err(MyError::IO(io::Error::new(
            io::ErrorKind::NotFound,
            it.display().to_string(),
        )));
    }

    Ok(it.display().to_string())
}

/// Return the path to the WASM plugins folder.
fn plugins_dir() -> PathBuf {
    PathBuf::from(format!("{}/plugins", SERVER_DIR))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name() -> Result<(), MyError> {
        // Test Vectors to ensure algos are working as expected...
        const XX_TV: u32 = 2082339723;
        const FX_TV: u32 = 2563774142;

        // we only need 1 of those PER THREAD...
        let mut pm = PluginMgr::default();

        // load plugins/modules...
        pm.load("xx")?;
        pm.load("fx")?;

        // ready...
        let seed: u32 = 100;
        let data = "1 if by land, 2 if by sea".as_bytes();

        let res_xx = pm.do_hash("xx", seed, data)?;
        let res_fx = pm.do_hash("fx", seed, data)?;

        println!("[DEBUG] res_xx = {}", res_xx);
        assert_eq!(res_xx, XX_TV);
        println!("[DEBUG] res_fx = {}", res_fx);
        assert_eq!(res_fx, FX_TV);

        let res_xx2 = pm.do_hash("xx", seed, data)?;
        println!("[DEBUG] res_xx2 = {}", res_xx2);
        assert_eq!(res_xx, res_xx2);

        Ok(())
    }
}
