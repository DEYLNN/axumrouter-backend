// Manual providers — custom logic that doesn't fit apikey/ or oauth/ patterns.
// Each subfolder is a self-contained provider with its own auth + client logic.
//
// To add a new manual provider:
// 1. Create `manual/<name>/` with mod.rs, constants.rs, auth.rs, client.rs, provider.rs
// 2. Add `pub mod <name>;` below
// 3. Register in `registry.rs`: register_provider!(registry, "xx", ...::XxProvider::new_with_keys, db);
// 4. FE label will be "Manual" (category = "manual")

pub mod unsloth;
