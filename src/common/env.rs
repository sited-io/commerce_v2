pub fn get_env_var_str(var: &str) -> String {
    std::env::var(var).unwrap_or_else(|_| {
        panic!("ERROR: Missing environment variable '{var}'")
    })
}

pub fn get_env_var_int(var: &str) -> usize {
    get_env_var_str(var).parse().unwrap_or_else(|_| {
        panic!("ERROR: Environment variable '{var}' was not an integer")
    })
}
