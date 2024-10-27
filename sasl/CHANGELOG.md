Version NEXT, released 20??-??-??:
	* Improvements
		- This crate is now `no_std`, you can use it even on platforms which don’t provide the `std` crate.

Version 0.5.2, released 2024-07-22:
	* Improvements
		- Add SCRAM client extensions support (thanks to Lucas Kent)
		- Update to edition 2021
		- Add tls-exporter channel binding
		- Use the right name for SCRAM with channel binding
		- Remove `ignore` keyword from doc-tests
		- Swap sha-1 dep to sha1
		- Update dependencies
		- Fix clippy lints and compiler warnings
		- Remove unneeded allocation in `client::mechanism::scram::Scram::initial`

Version 0.5.1, released 2023-08-20:
  * Important changes
    - Move sasl-rs to the xmpp-rs repository at https://gitlab.com/xmpp-rs/xmpp-rs.
  * Small changes
    - Use module FQNs in macro (thanks Raman Hafiyatulin)
    - Fix SASL ANONYMOUS service side (#11)
    - Update LICENSE file to reflect 0.5.0 changes
    - Bump dependencies

Version 0.5.0, released 2021-01-12:
  * Important changes
    - Relicensed to MPL-2.0 from LGPL-3.0-or-later.
    - Made all of the errors into enums, instead of strings.
  * Small changes
    - Replaced rand\_os with getrandom.
    - Bumped all dependencies.

Version 0.4.2, released 2018-05-19:
  * Small changes
    - Marc-Antoine Perennou updated the openssl and base64 dependencies to 0.10.4 and 0.9.0 respectively.
    - lumi updated them further to 0.10.7 and 0.9.1 respectively.
