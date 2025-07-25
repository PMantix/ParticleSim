use crate::config;

fn test_config_field() {
    let config = config::LJ_CONFIG.lock();
    let _k_e = config.coulomb_constant;
    println!("Coulomb constant: {}", _k_e);
}
