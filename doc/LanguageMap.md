A dictionary where the _key_ is an [RFC 5646][1] _Language Tag_, and the
_value_ is a string in the language indicated by the tag. This map is
supposed to be populated as fully as possible.

**IMPLEMENTATION NOTE** - This implementation uses a B-Tree based natural
language ordered map (`BTreeMap`), in preference to a `HashMap` b/c it
[seems to be][2] faster when deserializing, as well easier when finding
correct candidate for a langauge-tag w/ a country variant; for example
deciding on the appropriate label when the dictionary contains entries for
`en-US`, `en`, and `en-AU`.


# Requirements for _Canonical_ format

[xAPI requirements][3] for producing resources that have a [LanguageMap]
property when **`canonical`** format is specified state:

* [Activity][4] objects &mdash;more specifically their [ActivityDefinition][5]
  parts&mdash; contain [LanguageMap] objects within their `name`, `description`
  and various [InteractionComponent][6]s. The LRS **shall return only one
  language in each of these maps**.
* The LRS _may_ maintain canonical versions of language maps against any
  IRI identifying an object containing language maps. This includes the
  language map stored in the [Verb][7]'s `display` property and potentially
  some language maps used within extensions.
* The LRS _may_ maintain a canonical version of any language map and return
  this when **`canonical`** format is used to retrieve Statements. The LRS
  shall return only one language within each language map for which it
  returns a canonical map.
* In order to choose the most relevant language, the LRS shall apply the
  **`Accept-Language`** header as described in RFC-2616, except that this
  logic shall be applied to each language map individually to select which
  language entry to include, rather than to the resource (list of Statements)
  as a whole.


[1]: https://www.rfc-editor.org/rfc/rfc5646.html
[2]: https://users.rust-lang.org/t/hashmap-vs-btreemap/13804

[3]: https://opensource.ieee.org/xapi/xapi-base-standard-documentation/-/blob/main/9274.1.xAPI%20Base%20Standard%20for%20LRSs.md#language-filtering-requirements-for-canonical-format-statements
[4]: crate::Activity
[5]: crate::ActivityDefinition
[6]: crate::InteractionComponent
[7]: crate::Verb
