// =========================================================================
// STRICT COMPLIANCE BEVY 0.18.1 API:
// 1. USE .single() / .single_mut() -> returns Result<T, QuerySingleError>.
// 2. USE cmd.get_entity(e) -> returns Result<EntityCommands, InvalidEntityError>.
// 3. USE Without<T> filters to avoid B0001 (Mutable Aliasing Conflict).
// 4. Transform is a shared component, so Queries in the same system 
//    MUST be disjoint if one is mutable.
// =========================================================================
