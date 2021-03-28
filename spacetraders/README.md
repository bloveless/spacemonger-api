# SpaceTraders

This is a rust API wrapper for https://spacetraders.io

When running without optimizations (I.E. without --release) the library will not error if there are extra fields in the
JSON responses. In debug mode if the JSON contains extra fields an error will be emitted. This is done intentionally to hopefuly provide the most
correct client possible. If you come across one of
these errors please submit a PR to add the additional fields! Thanks!
