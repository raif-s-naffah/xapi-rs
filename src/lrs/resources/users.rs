// SPDX-License-Identifier: GPL-3.0-or-later

#![doc = include_str!("../../../doc/EXT_USERS.md")]

use crate::{
    DataError, MyError,
    db::user::{
        batch_update_users, find_all_ids, find_group_member_ids, find_group_user, find_user,
        insert_user, update_user,
    },
    emit_response, eval_preconditions,
    lrs::{
        DB, Headers, Role, User, etag_from_str, resources::WithResource,
        server::get_consistent_thru,
    },
};
use chrono::SecondsFormat;
use rocket::{
    FromForm, Route, State,
    form::Form,
    futures::TryFutureExt,
    get,
    http::{Header, Status, hyper::header},
    post, put, routes,
    serde::json::Json,
};
use tracing::{debug, info};

/// Form to use when creating new users.
#[derive(Debug, FromForm)]
struct CreateForm<'a> {
    email: &'a str,
    password: &'a str,
    /// Even root cannot create a user w/ Root (4) role!
    #[field(validate = range(0..4))]
    role: u16,
}

#[derive(Debug, FromForm)]
#[field(validate = range(0..4))]
pub(crate) struct RoleUI(pub(crate) u16);

/// Form to use when updating a single User.
#[derive(Debug, FromForm)]
pub(crate) struct UpdateForm<'a> {
    pub(crate) enabled: Option<bool>,
    pub(crate) email: Option<&'a str>,
    pub(crate) password: Option<&'a str>,
    pub(crate) role: Option<RoleUI>,
    #[field(name = uncased("managerId"))]
    pub(crate) manager_id: Option<i32>,
}

/// Form to use when updating multiple Users.
#[derive(Debug, FromForm)]
pub(crate) struct BatchUpdateForm {
    pub(crate) ids: Vec<i32>,
    pub(crate) enabled: Option<bool>,
    pub(crate) role: Option<RoleUI>,
    #[field(name = uncased("managerId"))]
    pub(crate) manager_id: Option<i32>,
}

#[doc(hidden)]
pub fn routes() -> Vec<Route> {
    routes![post, get_one, get_ids, update_one, update_many]
}

/// Create a new user w/ given properties. Newly created user is enabled and has
/// its `manager_id` field assigned the ID of the authenticated user making the
/// request.
///
/// Bare in mind that while _Root_ can assign any _Role, an _Admin_ can only
/// assign _User_ or _AuthUser_ roles.
///
/// When successful the response has a `200` _Status_ and contains _Etag_ and
/// _Last Modified_ headers.
#[post("/", data = "<form>")]
async fn post(
    form: Form<CreateForm<'_>>,
    db: &State<DB>,
    user: User,
) -> Result<WithResource<User>, MyError> {
    debug!("----- post ----- {}", user);
    user.can_manage_users()?;

    // validate new user attributes...
    // our user's Role can be one of two: 'root' or 'admin'.  new user's Role
    // depends on which one it is.
    let z_role = Role::from(form.role);
    // root can create users w/ any role except Root.  Rocket validation
    // annotation ensures it's never Root; i.e. the range's upper bound...
    if !user.is_root() {
        // it's an admin.  they can create users w/ User | AuthUser roles only
        if !matches!(z_role, Role::User | Role::AuthUser) {
            return Err(MyError::HTTP {
                status: Status::Forbidden,
                info: format!("Admin ({user}) can only create users w/ [Auth]User roles").into(),
            });
        }
    }
    let x = insert_user(db.pool(), (form.email, form.password, z_role, user.id)).await?;
    emit_user_response(x, false).await
}

/// Fetch the user w/ the designated ID if it exists.
///
/// Note though that if the authenticated user making the request is not _Root_,
/// but only an _Admin_ and the targeted user was found but is managed by a
/// different _Admin_, then the call will fail w/ a 404 _Status_.
#[get("/<id>")]
async fn get_one(id: i32, db: &State<DB>, user: User) -> Result<WithResource<User>, MyError> {
    debug!("----- get_one ----- {}", user);
    user.can_manage_users()?;

    if user.is_root() {
        let x = find_user(db.pool(), id)
            .map_err(|x| x.with_status(Status::NotFound))
            .await?;
        match x {
            Some(y) => emit_response!(Headers::default(), y => User),
            None => Err(MyError::HTTP {
                status: Status::NotFound,
                info: format!("User #{id} not found").into(),
            }),
        }
    } else if user.is_admin() {
        let x = find_group_user(db.pool(), id, user.id)
            .map_err(|x| x.with_status(Status::NotFound))
            .await?;
        match x {
            Some(y) => emit_response!(Headers::default(), y => User),
            None => Err(MyError::HTTP {
                status: Status::NotFound,
                info: format!("User #{id} not found").into(),
            }),
        }
    } else {
        Err(MyError::HTTP {
            status: Status::Forbidden,
            info: "Only Root and Admins can fetch users".into(),
        })
    }
}

/// Fetch all user IDs managed by the requesting authenticated user if they
/// are an _Admin_ or simply all user IDs if it was _Root_.
#[get("/")]
async fn get_ids(db: &State<DB>, user: User) -> Result<Json<Vec<i32>>, MyError> {
    debug!("----- get_ids ----- {}", user);
    user.can_manage_users()?;

    if user.is_root() {
        let x = find_all_ids(db.pool())
            .map_err(|x| x.with_status(Status::NotFound))
            .await?;
        Ok(Json(x))
    } else if user.is_admin() {
        let x = find_group_member_ids(db.pool(), user.id)
            .map_err(|x| x.with_status(Status::NotFound))
            .await?;
        Ok(Json(x))
    } else {
        Err(MyError::HTTP {
            status: Status::Forbidden,
            info: "Only Root and Admins can fetch users IDs".into(),
        })
    }
}

/// Update `enabled` flag, `email`, `password`, `role` or `manager_id` properties
/// for a single user given their ID.
///
/// _Roots_ as usual can modify any property for any user except themselves.
/// Every other _Role_, incl. _Guests_, can modify their `email` and `password`
/// properties. _Admins_ can only modify `enabled` and `role` for users they
/// manage. When changing `role`, _Admins_ can only toggle it between _User_
/// and _AuthUser_.
#[put("/<id>", data = "<form>")]
async fn update_one(
    c: Headers,
    id: i32,
    form: Form<UpdateForm<'_>>,
    db: &State<DB>,
    user: User,
) -> Result<WithResource<User>, MyError> {
    debug!("----- update_one ----- {user:?}");
    debug!("form = {form:?}");

    let x = find_user(db.pool(), id)
        .map_err(|x| x.with_status(Status::NotFound))
        .await?;
    let old_user = match x {
        Some(y) => y,
        None => {
            return Err(MyError::HTTP {
                status: Status::NotFound,
                info: format!("User #{id} not found").into(),
            });
        }
    };
    debug!("old_user = {old_user:?}");
    if old_user.is_root() {
        return Err(MyError::HTTP {
            status: Status::BadRequest,
            info: "Root properties are immutable".into(),
        });
    }

    // we do not allow combined updates --except for the special case of the
    // (email, password) pair b/c we do not store the raw password, only the
    // credentials which are computed from both...
    if form.enabled.is_some_and(|x| x != old_user.enabled) {
        // only root and current admin can alter enabled flag...
        if !(user.is_root() || user.id == old_user.manager_id) {
            return Err(MyError::HTTP {
                status: Status::BadRequest,
                info: "Only Root and the user's Admin can alter enabled flag".into(),
            });
        }
        debug!("Will update enabled flag...")
    } else if form
        .role
        .as_ref()
        .is_some_and(|x| Role::from(x.0) != old_user.role)
    {
        // Root can always re-assign roles.  Admins however can downgrade
        // (AuthUser -> User) or upgrade (User -> AuthUser) a role only for
        // users they manage...
        if !user.is_root() {
            if user.id != old_user.manager_id {
                return Err(MyError::HTTP {
                    status: Status::Forbidden,
                    info: "Only Root and the user's Admin can alter roles".into(),
                });
            }

            let new_role = Role::from(form.role.as_ref().unwrap().0);
            if !matches!(new_role, Role::User | Role::AuthUser) {
                return Err(MyError::HTTP {
                    status: Status::BadRequest,
                    info: "Admins can alter roles from User to AuthUser or vice-versa only".into(),
                });
            }
        }
        debug!("Will update role...")
    } else if form.manager_id.is_some_and(|x| x != old_user.manager_id) {
        // only root can re-assign manager_id...
        if !user.is_root() {
            return Err(MyError::HTTP {
                status: Status::BadRequest,
                info: "Only Root can alter manager_id".into(),
            });
        }
        debug!("Will update manager_id...")
    } else if form.email.is_some() || form.password.is_some() {
        // both must be provided...
        if form.email.is_none() || form.password.is_none() {
            return Err(MyError::HTTP {
                status: Status::BadRequest,
                info: "When updating either 'email' or 'password' both values must be provided"
                    .into(),
            });
        }
        // only a non-root user can change their email and/or password...
        if user.is_root() || user.id != id {
            return Err(MyError::HTTP {
                status: Status::BadRequest,
                info: "Only non-Root user can alter their 'email' + 'password' fields".into(),
            });
        }
        debug!("Will update email + credentials...")
    } else {
        return Err(MyError::HTTP {
            status: Status::BadRequest,
            info: "You're wasting my time :(".into(),
        });
    }

    // continue if pre-conditions exist + pass...
    if c.has_no_conditionals() {
        Err(MyError::HTTP {
            status: Status::Conflict,
            info: "Update User w/ no pre-conditions is NOT allowed".into(),
        })
    } else {
        let x = serde_json::to_string(&old_user).map_err(|x| MyError::Data(DataError::JSON(x)))?;
        let etag = etag_from_str(&x);
        debug!("etag (old) = {}", etag);
        match eval_preconditions!(&etag, c) {
            s if s != Status::Ok => Err(MyError::HTTP {
                status: s,
                info: "Failed pre-condition(s)".into(),
            }),
            _ => {
                let x = update_user(db.pool(), id, form.into_inner()).await?;
                emit_user_response(x, true).await
            }
        }
    }
}

/// Batch modification of `enabled` flag, `role` and `manager_id` for a limited
/// set of users given their IDs.
///
/// If the authenticated user making the request is an _Admin_ then the users
/// targeted must be already managed by them. In addition, _Admins_ can only
/// batch modify the first two properties; not the `manager_id`. That last
/// one is only possible with _Root_.
///
/// The response will include the IDs of the users that were successfully
/// modified.
#[put("/", data = "<form>")]
async fn update_many(
    form: Form<BatchUpdateForm>,
    db: &State<DB>,
    user: User,
) -> Result<Status, MyError> {
    debug!("----- update_many ----- {user:?}");
    user.can_manage_users()?;

    debug!("form = {form:?}");

    // if IDs array is empty return...
    let ids = &form.ids;
    if ids.is_empty() {
        info!("Empty user IDs array. Do nothing");
        return Ok(Status::Ok);
    }

    let conn = db.pool();
    // if user is Admin, ensures all IDs are for users they manage...
    if user.is_admin() {
        let x = find_group_member_ids(conn, user.id).await?;
        let ok = ids.iter().all(|id| x.contains(id));
        if !ok {
            return Err(MyError::HTTP {
                status: Status::BadRequest,
                info: "Admins can only do batch updates for users they manage".into(),
            });
        }

        // Admins can only change Role to User or AuthUser...
        if form
            .role
            .as_ref()
            .is_some_and(|x| !matches!(Role::from(x.0), Role::User | Role::AuthUser))
        {
            return Err(MyError::HTTP {
                status: Status::BadRequest,
                info: "Admins can only toggle role between User and AuthUser".into(),
            });
        }

        // only Root can alter manager_id...
        if form.manager_id.is_some() {
            return Err(MyError::HTTP {
                status: Status::Forbidden,
                info: "Only Root can re-assign manager ID".into(),
            });
        }
    }

    batch_update_users(db.pool(), form.into_inner()).await?;
    // NOTE (rsn) 20250317 - the safest course of action here is to
    // clear the LRU cache.
    User::clear_cache().await;

    Ok(Status::Ok)
}

/// Construct and return a response based on the User `u`. If in addition,
/// `uncache` is TRUE then also evict said User from the LRU cache.
async fn emit_user_response(u: User, uncache: bool) -> Result<WithResource<User>, MyError> {
    let x = serde_json::to_string(&u).map_err(|x| MyError::Data(DataError::JSON(x)))?;
    let etag = etag_from_str(&x);
    debug!("etag (new) = {}", etag);
    let last_modified = get_consistent_thru()
        .await
        .to_rfc3339_opts(SecondsFormat::Millis, true);

    if uncache {
        u.uncache().await
    }

    Ok(WithResource {
        inner: rocket::serde::json::Json(u),
        etag: Header::new(header::ETAG.as_str(), etag.to_string()),
        last_modified: Header::new(header::LAST_MODIFIED.as_str(), last_modified),
    })
}
