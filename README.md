# adblock-rust-server
Server wrapper for the [adblock crate](https://crates.io/crates/adblock) using Unix domain sockets.

Created for [blockit](https://github.com/dudik/blockit), but can be used in any program that supports Unix domain sockets.

## Usage
Add urls of filter lists (e.g. https://easylist.to/easylist/easylist.txt) to `~/.config/ars/urls`.  
Custom rules (e.g. ###customAd) should be added to `~/.config/ars/lists/custom`.

## API
After launching adblock-rust-server connect to `/tmp/ars` socket file to start communicating. 
Every request and response message have to end with a new line character `\n`. Two request types are supported:

### Network request
`n <request_url> <source_url> <request_type>`

For example:
`n https://duckduckgo.com/p103.js https://duckduckgo.com/ script`

Checks if the request should be blocked. Returns `1` if it should and `0` if not.

### Cosmetic request
`c <website_url> <ids> <classes>`

For example:
`c https://duckduckgo.com/ pg-index   wedonttrack content_homepage    logo_homepage_link`

Returns a CSS rule that hides unwanted elements. `<ids>` and `<classes>` are lists of id/class names separated by the tab character `\t`.

### Reload engine request
`r`
Restarts the adblock-rust engine applying any changes made to fiter lists or custom rules.

### Force update request
`u`
Updates every filter list and restarts the engine.
