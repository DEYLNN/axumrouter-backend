// Provider admin endpoints — split per concern.
//
// list    — GET /admin/api/providers
// detail  — GET /admin/api/providers/:id
// validate ��� GET /admin/api/providers/:id/validate-models
// test    — POST /admin/api/providers/:id/test
// block   — POST /admin/api/providers/:id/{block,unblock}

pub mod block;
pub mod custom_models;
pub mod detail;
pub mod list;
pub mod test;
pub mod validate;

pub use block::{api_block_model, api_unblock_model};
pub use custom_models::{api_list_custom_models, api_add_custom_model_for_provider, api_remove_custom_model_for_provider};
pub use detail::api_provider_detail;
pub use list::api_providers;
pub use test::api_test_model;
pub use validate::api_validate_models;
