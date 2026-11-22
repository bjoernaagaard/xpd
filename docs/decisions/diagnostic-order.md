# Keep diagnostic ordering stable when compiling packs in parallel

Status. Accepted, describing the implementation at 8f1403b.

Compiling rules contributes to startup time. In `Pack::try_from` the embedded files compile in parallel. Collection into a vector and flattening preserve file order. Scan reporting uses a stable sort by source position so findings at the same position retain their pack and rule order. This allows parallel compilation without changing that part of the user-visible report.

Evidence. See `src/pack.rs` at lines 251–262 and `src/scan.rs` at lines 437–440 of 8f1403b.
