//! Shared raw GUID/hash candidate grouping for mesh and material navigation.
pub(super) fn reference_groups(
    by_guid: &std::collections::HashMap<String, Vec<usize>>,
    by_hash: &std::collections::HashMap<String, Vec<usize>>,
    guid: Option<u32>,
    hash: Option<&str>,
) -> Vec<(String, Vec<usize>)> {
    let mut groups = Vec::new();
    if let Some(g) = guid {
        let id = format!("{g:08x}");
        let targets = if g == 0 {
            vec![]
        } else {
            by_guid.get(&id).cloned().unwrap_or_default()
        };
        groups.push((format!("GUID {id}"), targets));
    }
    if let Some(h) = hash {
        let targets = if h.chars().all(|c| c == '0') {
            vec![]
        } else {
            by_hash
                .get(&h.to_ascii_lowercase())
                .cloned()
                .unwrap_or_default()
        };
        groups.push((format!("SHA1 {h}"), targets));
    }
    groups
}
