use rand::rngs::OsRng;
use rsa::{
    pkcs1::EncodeRsaPublicKey,
    pkcs8::{DecodePrivateKey, EncodePrivateKey},
    RsaPrivateKey,
};
use zeroize::Zeroizing;
use zsign_gui_core::sessions::SecretStore;

fn key_name(team_id: &str) -> Result<String, String> {
    if team_id.len() != 10
        || !team_id
            .bytes()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
    {
        return Err("开发团队 ID 无效".into());
    }
    Ok(format!("signing-key:{team_id}"))
}

pub fn load_signing_key<S: SecretStore>(
    store: &S,
    team_id: &str,
) -> Result<Option<RsaPrivateKey>, String> {
    let Some(bytes) = store.get(&key_name(team_id)?)? else {
        return Ok(None);
    };
    let bytes = Zeroizing::new(bytes);
    RsaPrivateKey::from_pkcs8_der(&bytes)
        .map(Some)
        .map_err(|_| "已保存的签名私钥无效，未覆盖原有数据".into())
}

pub fn ensure_signing_key<S: SecretStore>(
    store: &S,
    team_id: &str,
) -> Result<RsaPrivateKey, String> {
    if let Some(key) = load_signing_key(store, team_id)? {
        return Ok(key);
    }
    let key = RsaPrivateKey::new(&mut OsRng, 2048).map_err(|_| "无法生成签名私钥")?;
    let encoded = key.to_pkcs8_der().map_err(|_| "无法编码签名私钥")?;
    store.set(&key_name(team_id)?, encoded.as_bytes())?;
    Ok(key)
}

pub fn certificate_matches(key: &RsaPrivateKey, certificate_der: &[u8]) -> Result<bool, String> {
    let certificate = x509_certificate::X509Certificate::from_der(certificate_der)
        .map_err(|_| "无法读取开发证书")?;
    let public_key = key
        .to_public_key()
        .to_pkcs1_der()
        .map_err(|_| "无法读取签名公钥")?;
    Ok(public_key.as_bytes() == certificate.public_key_data().as_ref())
}

pub fn certificate_request(key: &RsaPrivateKey) -> Result<String, String> {
    let der = key.to_pkcs8_der().map_err(|_| "无法编码签名私钥")?;
    let mut params = rcgen::CertificateParams::new(vec![]);
    params.alg = &rcgen::PKCS_RSA_SHA256;
    params.key_pair =
        Some(rcgen::KeyPair::from_der(der.as_bytes()).map_err(|_| "无法创建证书请求")?);
    params
        .distinguished_name
        .push(rcgen::DnType::CommonName, "zsign Desktop");
    rcgen::Certificate::from_params(params)
        .and_then(|cert| cert.serialize_request_pem())
        .map_err(|_| "无法生成证书请求".into())
}

pub async fn prepare_certificate(
    session: &plume_core::developer::DeveloperSession,
    team: &String,
    create_allowed: bool,
) -> Result<(RsaPrivateKey, Vec<u8>), String> {
    let key = if create_allowed {
        ensure_signing_key(&crate::secrets::SystemSecretStore, team)?
    } else {
        load_signing_key(&crate::secrets::SystemSecretStore, team)?
            .ok_or("已预览的签名私钥不可用，请重新检查")?
    };
    let certs = session
        .qh_list_certs(team)
        .await
        .map_err(|e| e.to_string())?
        .certificates;
    for cert in &certs {
        let Some(content) = &cert.cert_content else {
            continue;
        };
        let expires: std::time::SystemTime = cert.expiration_date.clone().into();
        if expires > std::time::SystemTime::now()
            && (cert.status.eq_ignore_ascii_case("issued")
                || cert.status.eq_ignore_ascii_case("active"))
            && certificate_matches(&key, content.as_ref())?
        {
            return Ok((key, content.as_ref().to_vec()));
        }
    }
    if !create_allowed {
        return Err("已预览的证书不再可用，请重新检查".into());
    }
    let request = certificate_request(&key)?;
    let created = session
        .qh_submit_cert_csr(team, request, &"zsign Desktop".to_string())
        .await
        .map_err(|e| format!("无法创建开发证书（不会自动撤销已有证书）：{e}"))?
        .cert_request;
    let certs = session
        .qh_list_certs(team)
        .await
        .map_err(|e| e.to_string())?
        .certificates;
    let cert = certs
        .into_iter()
        .find(|c| c.certificate_id == created.certificate_id)
        .and_then(|c| c.cert_content)
        .ok_or("证书已申请，但 Apple 尚未返回证书内容，请重新检查")?;
    if !certificate_matches(&key, cert.as_ref())? {
        return Err("Apple 返回的证书与私钥不匹配".into());
    }
    Ok((key, cert.as_ref().to_vec()))
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Mutex};
    use zsign_gui_core::sessions::SecretStore;

    #[derive(Default)]
    struct MemoryStore(Mutex<HashMap<String, Vec<u8>>>);
    impl SecretStore for MemoryStore {
        fn set(&self, key: &str, value: &[u8]) -> Result<(), String> {
            self.0.lock().unwrap().insert(key.into(), value.into());
            Ok(())
        }
        fn get(&self, key: &str) -> Result<Option<Vec<u8>>, String> {
            Ok(self.0.lock().unwrap().get(key).cloned())
        }
        fn delete(&self, key: &str) -> Result<(), String> {
            self.0.lock().unwrap().remove(key);
            Ok(())
        }
    }

    #[test]
    fn a_team_signing_key_is_reused_on_a_later_request() {
        let store = MemoryStore::default();
        let first = super::ensure_signing_key(&store, "TEAM123456").unwrap();
        let second = super::ensure_signing_key(&store, "TEAM123456").unwrap();
        assert_eq!(first.to_public_key(), second.to_public_key());
        assert_eq!(store.0.lock().unwrap().len(), 1);
    }

    #[test]
    fn a_certificate_matches_only_its_own_signing_key() {
        use rsa::pkcs8::EncodePrivateKey;
        let store = MemoryStore::default();
        let first = super::ensure_signing_key(&store, "TEAM123456").unwrap();
        let second = super::ensure_signing_key(&store, "TEAM654321").unwrap();
        let key_der = first.to_pkcs8_der().unwrap();
        let mut params = rcgen::CertificateParams::new(vec![]);
        params.alg = &rcgen::PKCS_RSA_SHA256;
        params.key_pair = Some(rcgen::KeyPair::from_der(key_der.as_bytes()).unwrap());
        let cert = rcgen::Certificate::from_params(params)
            .unwrap()
            .serialize_der()
            .unwrap();
        assert!(super::certificate_matches(&first, &cert).unwrap());
        assert!(!super::certificate_matches(&second, &cert).unwrap());
    }

    #[test]
    fn a_corrupt_saved_key_is_not_overwritten() {
        let store = MemoryStore::default();
        store.set("signing-key:TEAM123456", b"not a key").unwrap();
        assert!(super::ensure_signing_key(&store, "TEAM123456").is_err());
        assert_eq!(
            store.get("signing-key:TEAM123456").unwrap().unwrap(),
            b"not a key"
        );
    }
}
