pub fn get_names() -> Vec<String> {
    #[cfg(debug_assertions)]
    {
        vec!["VK_LAYER_KHRONOS_validation".to_string()]
    }
    #[cfg(not(debug_assertions))]
    {
        vec![]
    }
}
