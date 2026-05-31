# Fields included in `Format` variants

The next table shows for each [Format] variant, except **`exact`**, which fields
are included when generating a response to a **`GET`** _Statement_ resource call.
The reason **`exact`** is not shown is b/c in that case **_all_** fields are
included.

<table>
    <thead>
        <tr><th>Entity</th><th>Field</th><th><code>ids</code></th><th><code>canonical</code></th></tr>
    </thead>
    <tbody>
        <tr><td>Statement</td>  <td>id</td>         <td>✓</td><td>✓</td></tr>
        <tr><td></td>           <td>actor</td>      <td>✓</td><td>✓</td></tr>
        <tr><td></td>           <td>verb</td>       <td>✓</td><td>✓</td></tr>
        <tr><td></td>           <td>object</td>     <td>✓</td><td>✓</td></tr>
        <tr><td></td>           <td>result</td>     <td> </td><td>✓</td></tr>
        <tr><td></td>           <td>context</td>    <td> </td><td>✓</td></tr>
        <tr><td></td>           <td>timestamp</td>  <td> </td><td> </td></tr>
        <tr><td></td>           <td>stored</td>     <td> </td><td> </td></tr>
        <tr><td></td>           <td>authority</td>  <td>✓</td><td>✓</td></tr>
        <tr><td></td>           <td>version</td>    <td> </td><td> </td></tr>
        <tr><td>Agent</td>  <td>id</td>         <td>✓</td><td>✓</td></tr>
        <tr><td></td>       <td>ifi [1, 2]</td> <td>✓</td><td>✓</td></tr>
        <tr><td>Group</td>  <td>id</td>         <td>✓</td><td>✓</td></tr>
        <tr><td></td>       <td>ifi [1, 2]</td> <td>✓</td><td>✓</td></tr>
        <tr><td></td>       <td>members</td>    <td>✓</td><td>✓</td></tr>
        <tr><td>Verb</td>   <td>id</td>         <td>✓</td><td>✓</td></tr>
        <tr><td></td>       <td>display [3]</td><td> </td><td>✓</td></tr>
        <tr><td>Activity</td>   <td>id</td>         <td>✓</td><td>✓</td></tr>
        <tr><td></td>           <td>definition</td> <td> </td><td>✓</td></tr>
        <tr><td>ActivityDefinition</td>  <td>name</td>                      <td> </td><td>✓</td></tr>
        <tr><td></td>                    <td>description</td>               <td> </td><td>✓</td></tr>
        <tr><td></td>                    <td>type_</td>                     <td> </td><td>✓</td></tr>
        <tr><td></td>                    <td>more_info</td>                 <td> </td><td>✓</td></tr>
        <tr><td></td>                    <td>interaction_type</td>          <td> </td><td>✓</td></tr>
        <tr><td></td>                    <td>correct_responses_pattern</td> <td> </td><td>✓</td></tr>
        <tr><td></td>                    <td>choices</td>                   <td> </td><td>✓</td></tr>
        <tr><td></td>                    <td>scale</td>                     <td> </td><td>✓</td></tr>
        <tr><td></td>                    <td>source</td>                    <td> </td><td>✓</td></tr>
        <tr><td></td>                    <td>target</td>                    <td> </td><td>✓</td></tr>
        <tr><td></td>                    <td>steps</td>                     <td> </td><td>✓</td></tr>
        <tr><td></td>                    <td>extensions</td>                <td> </td><td>✓</td></tr>
        <tr><td>InteractionComponent</td><td>id</td>            <td> </td><td>✓</td></tr>
        <tr><td></td>                    <td>description</td>   <td> </td><td>✓</td></tr>
        <tr><td>StatementRef</td>  <td>id</td><td>✓</td><td>✓</td></tr>
        <tr><td>SubStatement</td>   <td>actor</td>      <td>✓</td><td>✓</td></tr>
        <tr><td></td>               <td>verb</td>       <td>✓</td><td>✓</td></tr>
        <tr><td></td>               <td>object</td>     <td>✓</td><td>✓</td></tr>
        <tr><td></td>               <td>result</td>     <td> </td><td>✓</td></tr>
        <tr><td></td>               <td>context</td>    <td> </td><td>✓</td></tr>
        <tr><td></td>               <td>timestamp</td>  <td> </td><td> </td></tr>
    </tbody>
</table>

Notes:
1. The term **`ifi`** refers to either one of the following four _Inverse
   Functional Identifier_ fields: `mbox`, `mbox_sha1sum`, `openid` or `account`.
2. With **`ids`** and **`canonical`** variants, only one (1) IFI is included
   in the output. However, with **`exact`** all known IFIs are included.
3. **`display`** like all [LanguageMap][2] instances are included in **`canonical`**
   with only one (1) language tag entry. With **`exact`** again all entries are
   included. For more information read [LanguageMap Requirements](crate::LanguageMap#requirements-for-canonical-format)

[2]: crate::LanguageMap
