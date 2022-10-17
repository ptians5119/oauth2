# Details

Date : 2022-09-27 16:15:31

Directory d:\\Projects\\oauth2\\oxide-auth\\src

Total : 44 files,  7605 codes, 2542 comments, 1536 blanks, all 11683 lines

[Summary](results.md) / Details / [Diff Summary](diff.md) / [Diff Details](diff-details.md)

## Files
| filename | language | code | comment | blank | total |
| :--- | :--- | ---: | ---: | ---: | ---: |
| [src/code_grant/accesstoken.rs](/src/code_grant/accesstoken.rs) | Rust | 467 | 167 | 81 | 715 |
| [src/code_grant/authorization.rs](/src/code_grant/authorization.rs) | Rust | 415 | 111 | 56 | 582 |
| [src/code_grant/error.rs](/src/code_grant/error.rs) | Rust | 207 | 77 | 47 | 331 |
| [src/code_grant/extensions/mod.rs](/src/code_grant/extensions/mod.rs) | Rust | 2 | 1 | 2 | 5 |
| [src/code_grant/extensions/pkce.rs](/src/code_grant/extensions/pkce.rs) | Rust | 117 | 47 | 23 | 187 |
| [src/code_grant/mod.rs](/src/code_grant/mod.rs) | Rust | 6 | 28 | 2 | 36 |
| [src/code_grant/refresh.rs](/src/code_grant/refresh.rs) | Rust | 353 | 167 | 50 | 570 |
| [src/code_grant/resource.rs](/src/code_grant/resource.rs) | Rust | 286 | 84 | 53 | 423 |
| [src/endpoint/accesstoken.rs](/src/endpoint/accesstoken.rs) | Rust | 227 | 42 | 46 | 315 |
| [src/endpoint/authorization.rs](/src/endpoint/authorization.rs) | Rust | 288 | 56 | 53 | 397 |
| [src/endpoint/error.rs](/src/endpoint/error.rs) | Rust | 18 | 19 | 6 | 43 |
| [src/endpoint/mod.rs](/src/endpoint/mod.rs) | Rust | 307 | 283 | 95 | 685 |
| [src/endpoint/query.rs](/src/endpoint/query.rs) | Rust | 247 | 47 | 47 | 341 |
| [src/endpoint/refresh.rs](/src/endpoint/refresh.rs) | Rust | 185 | 26 | 35 | 246 |
| [src/endpoint/resource.rs](/src/endpoint/resource.rs) | Rust | 111 | 23 | 26 | 160 |
| [src/endpoint/tests/access_token.rs](/src/endpoint/tests/access_token.rs) | Rust | 507 | 19 | 74 | 600 |
| [src/endpoint/tests/authorization.rs](/src/endpoint/tests/authorization.rs) | Rust | 251 | 7 | 37 | 295 |
| [src/endpoint/tests/mod.rs](/src/endpoint/tests/mod.rs) | Rust | 156 | 28 | 40 | 224 |
| [src/endpoint/tests/pkce.rs](/src/endpoint/tests/pkce.rs) | Rust | 184 | 2 | 32 | 218 |
| [src/endpoint/tests/refresh.rs](/src/endpoint/tests/refresh.rs) | Rust | 322 | 7 | 57 | 386 |
| [src/endpoint/tests/resource.rs](/src/endpoint/tests/resource.rs) | Rust | 128 | 6 | 25 | 159 |
| [src/frontends/actix.rs](/src/frontends/actix.rs) | Rust | 0 | 0 | 2 | 2 |
| [src/frontends/gotham.rs](/src/frontends/gotham.rs) | Rust | 292 | 81 | 63 | 436 |
| [src/frontends/iron.rs](/src/frontends/iron.rs) | Rust | 0 | 0 | 4 | 4 |
| [src/frontends/mod.rs](/src/frontends/mod.rs) | Rust | 7 | 175 | 3 | 185 |
| [src/frontends/rocket.rs](/src/frontends/rocket.rs) | Rust | 0 | 0 | 4 | 4 |
| [src/frontends/rouille.rs](/src/frontends/rouille.rs) | Rust | 0 | 0 | 4 | 4 |
| [src/frontends/simple/endpoint.rs](/src/frontends/simple/endpoint.rs) | Rust | 378 | 241 | 70 | 689 |
| [src/frontends/simple/extensions/extended.rs](/src/frontends/simple/extensions/extended.rs) | Rust | 68 | 9 | 17 | 94 |
| [src/frontends/simple/extensions/list.rs](/src/frontends/simple/extensions/list.rs) | Rust | 98 | 10 | 21 | 129 |
| [src/frontends/simple/extensions/mod.rs](/src/frontends/simple/extensions/mod.rs) | Rust | 80 | 24 | 19 | 123 |
| [src/frontends/simple/extensions/pkce.rs](/src/frontends/simple/extensions/pkce.rs) | Rust | 24 | 0 | 7 | 31 |
| [src/frontends/simple/mod.rs](/src/frontends/simple/mod.rs) | Rust | 3 | 9 | 3 | 15 |
| [src/frontends/simple/request.rs](/src/frontends/simple/request.rs) | Rust | 158 | 46 | 47 | 251 |
| [src/lib.rs](/src/lib.rs) | Rust | 8 | 65 | 4 | 77 |
| [src/primitives/authorizer.rs](/src/primitives/authorizer.rs) | Rust | 126 | 35 | 26 | 187 |
| [src/primitives/generator.rs](/src/primitives/generator.rs) | Rust | 244 | 80 | 60 | 384 |
| [src/primitives/grant.rs](/src/primitives/grant.rs) | Rust | 181 | 70 | 41 | 292 |
| [src/primitives/issuer.rs](/src/primitives/issuer.rs) | Rust | 393 | 167 | 97 | 657 |
| [src/primitives/mod.rs](/src/primitives/mod.rs) | Rust | 17 | 34 | 5 | 56 |
| [src/primitives/registrar.rs](/src/primitives/registrar.rs) | Rust | 538 | 161 | 109 | 808 |
| [src/primitives/scope.rs](/src/primitives/scope.rs) | Rust | 129 | 56 | 24 | 209 |
| [src/token_grant/authorization.rs](/src/token_grant/authorization.rs) | Rust | 76 | 4 | 16 | 96 |
| [src/token_grant/mod.rs](/src/token_grant/mod.rs) | Rust | 1 | 28 | 3 | 32 |

[Summary](results.md) / Details / [Diff Summary](diff.md) / [Diff Details](diff-details.md)