use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm,
};
use argon2::{
    password_hash::{rand_core::RngCore, SaltString},
    Argon2, PasswordHasher,
};
use base64::{engine::general_purpose, Engine as _};
use keyring::Entry;
use std::sync::OnceLock;

static MASTER_KEY: OnceLock<[u8; 32]> = OnceLock::new();

/// 初始化主密钥（从系统密钥链获取或生成）
pub fn initialize_master_key() -> Result<(), String> {
    MASTER_KEY.get_or_init(|| {
        // 尝试从系统密钥链获取
        match get_or_create_master_key() {
            Ok(key) => key,
            Err(e) => {
                eprintln!("密钥初始化警告: {}", e);
                // 回退到机器ID派生的密钥
                derive_machine_key()
            }
        }
    });
    Ok(())
}

/// 从系统密钥链获取或创建主密钥
fn get_or_create_master_key() -> Result<[u8; 32], String> {
    let service_name = "DataSmith";
    let username = "master_encryption_key";
    
    let entry = Entry::new(service_name, username)
        .map_err(|e| format!("无法访问系统密钥链: {}", e))?;

    match entry.get_password() {
        Ok(key_str) => {
            // 密钥存在，解码并返回
            let decoded = general_purpose::STANDARD
                .decode(&key_str)
                .map_err(|e| format!("密钥解码失败: {}", e))?;
            
            if decoded.len() != 32 {
                return Err("存储的密钥长度不正确".to_string());
            }
            
            let mut key = [0u8; 32];
            key.copy_from_slice(&decoded);
            Ok(key)
        }
        Err(_) => {
            // 密钥不存在，生成新密钥
            let mut key = [0u8; 32];
            OsRng.fill_bytes(&mut key);
            
            // 保存到系统密钥链
            let encoded = general_purpose::STANDARD.encode(&key);
            entry
                .set_password(&encoded)
                .map_err(|e| format!("无法保存密钥到系统密钥链: {}", e))?;
            
            Ok(key)
        }
    }
}

/// 使用机器ID派生密钥（备用方案）
fn derive_machine_key() -> [u8; 32] {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    // 获取机器唯一标识
    let machine_id = get_machine_id();
    
    // 使用Argon2派生密钥
    let salt = SaltString::from_b64("DataSmithSaltV1.0.0.0.0.0.0")
        .unwrap_or_else(|_| SaltString::from_b64("aaaaaaaaaaaaaaaaaaaaaa").unwrap());
    
    let argon2 = Argon2::default();
    
    match argon2.hash_password(machine_id.as_bytes(), &salt) {
        Ok(hash) => {
            let hash_output = hash.hash.unwrap();
            let hash_bytes = hash_output.as_bytes();
            let mut key = [0u8; 32];
            key.copy_from_slice(&hash_bytes[..32]);
            key
        }
        Err(_) => {
            // 最后的回退方案
            let mut hasher = DefaultHasher::new();
            machine_id.hash(&mut hasher);
            let hash = hasher.finish();
            let mut key = [0u8; 32];
            for (i, byte) in key.iter_mut().enumerate() {
                *byte = ((hash >> (i % 8)) ^ (i as u64)) as u8;
            }
            key
        }
    }
}

/// 获取机器唯一标识
fn get_machine_id() -> String {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        if let Ok(output) = Command::new("wmic")
            .args(&["csproduct", "get", "uuid"])
            .output()
        {
            if let Ok(id) = String::from_utf8(output.stdout) {
                return id.lines().nth(1).unwrap_or("default-machine-id").trim().to_string();
            }
        }
    }
    
    #[cfg(target_os = "linux")]
    {
        if let Ok(id) = std::fs::read_to_string("/etc/machine-id") {
            return id.trim().to_string();
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        if let Ok(output) = Command::new("ioreg")
            .args(&["-rd1", "-c", "IOPlatformExpertDevice"])
            .output()
        {
            if let Ok(id) = String::from_utf8(output.stdout) {
                return id;
            }
        }
    }
    
    // 回退到主机名
    match hostname::get() {
        Ok(name) => name.to_string_lossy().to_string(),
        Err(_) => "datasmith-default-machine".to_string(),
    }
}

/// 获取主密钥
fn get_master_key() -> Result<&'static [u8; 32], String> {
    MASTER_KEY
        .get()
        .ok_or_else(|| "主密钥未初始化，请先调用 initialize_master_key()".to_string())
}

/// 加密密码
pub fn encrypt_password(password: &str) -> Result<String, String> {
    let key = get_master_key()?;
    let cipher = Aes256Gcm::new(key.into());
    
    // 生成随机nonce
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = (&nonce_bytes).into();
    
    let ciphertext = cipher
        .encrypt(nonce, password.as_bytes())
        .map_err(|e| format!("加密失败: {}", e))?;
    
    // 将nonce和密文一起编码
    let mut result = Vec::with_capacity(nonce_bytes.len() + ciphertext.len());
    result.extend_from_slice(&nonce_bytes);
    result.extend_from_slice(&ciphertext);
    
    Ok(general_purpose::STANDARD.encode(result))
}

/// 解密密码
pub fn decrypt_password(encrypted: &str) -> Result<String, String> {
    let key = get_master_key()?;
    let cipher = Aes256Gcm::new(key.into());
    
    let data = general_purpose::STANDARD
        .decode(encrypted)
        .map_err(|e| format!("Base64 解码失败: {}", e))?;
    
    if data.len() < 12 {
        return Err("加密数据格式无效".to_string());
    }
    
    // 分离nonce和密文
    let (nonce_bytes, ciphertext) = data.split_at(12);
    let nonce = nonce_bytes.into();
    
    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| format!("解密失败: {}", e))?;
    
    String::from_utf8(plaintext).map_err(|e| format!("UTF-8 转换失败: {}", e))
}

