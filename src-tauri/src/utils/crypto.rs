use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm,
};
use argon2::password_hash::rand_core::RngCore;
use base64::{engine::general_purpose, Engine as _};
use std::sync::OnceLock;

static MASTER_KEY: OnceLock<[u8; 32]> = OnceLock::new();

/// 初始化主密钥（使用机器ID派生确定性密钥）
pub fn initialize_master_key() -> Result<(), String> {
    MASTER_KEY.get_or_init(|| {
        // 直接使用机器ID派生密钥，确保确定性和跨重启一致性
        derive_machine_key()
    });
    Ok(())
}

/// 使用机器ID派生密钥（备用方案，确定性派生）
fn derive_machine_key() -> [u8; 32] {
    use sha2::{Digest, Sha256};
    
    // 获取机器唯一标识
    let machine_id = get_machine_id();
    
    // 使用固定的 salt 确保确定性
    let salt = "DataSmithSaltV1.0.0.0.0.0.0";
    
    // 使用 SHA-256 进行确定性密钥派生
    let mut hasher = Sha256::new();
    hasher.update(machine_id.as_bytes());
    hasher.update(salt.as_bytes());
    let hash_result = hasher.finalize();
    
    let mut key = [0u8; 32];
    key.copy_from_slice(&hash_result);
    key
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

        // 首选：从 ioreg 输出中取 IOPlatformUUID
        if let Ok(output) = Command::new("ioreg")
            .args(&["-rd1", "-c", "IOPlatformExpertDevice"])
            .output()
        {
            if let Ok(text) = String::from_utf8(output.stdout) {
                for line in text.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("\"IOPlatformUUID\"") {
                        if let Some(eq_pos) = trimmed.find('=') {
                            let uuid = trimmed[eq_pos + 1..].trim().trim_matches('"').trim().to_string();
                            if !uuid.is_empty() {
                                return uuid;
                            }
                        }
                    }
                }
            }
        }

        // 备用：system_profiler 中读取 Hardware UUID
        if let Ok(output) = Command::new("system_profiler")
            .args(&["SPHardwareDataType"])
            .output()
        {
            if let Ok(text) = String::from_utf8(output.stdout) {
                for line in text.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("Hardware UUID:") {
                        let parts: Vec<&str> = trimmed.split(':').collect();
                        if parts.len() > 1 {
                            let uuid = parts[1].trim().to_string();
                            if !uuid.is_empty() {
                                return uuid;
                            }
                        }
                    }
                }
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
