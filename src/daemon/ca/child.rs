use std::collections::HashMap;

use chrono::Duration;

use rpki::crypto::KeyIdentifier;
use rpki::x509::Time;

use crate::commons::api::{ChildCaInfo, ChildHandle, IssuedCert, ResourceClassName, ResourceSet};
use crate::commons::crypto::IdCert;
use crate::commons::error::Error;
use crate::commons::KrillResult;
use crate::daemon::config::IssuanceTimingConfig;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[allow(clippy::large_enum_variant)]
#[serde(rename_all = "snake_case")]
pub enum LastResponse {
    Current(ResourceClassName),
    Revoked,
}

impl LastResponse {}

//------------ ChildInfo ---------------------------------------------------

/// Contains information about a child CA needed by a parent [CertAuth](ca.CertAuth).
///
/// Note that the actual [IssuedCert] corresponding to the [KeyIdentifier]
/// and [ResourceClassName] are kept in the parent's [ResourceClass].
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ChildDetails {
    id_cert: IdCert,
    resources: ResourceSet,
    used_keys: HashMap<KeyIdentifier, LastResponse>,
}

impl ChildDetails {
    pub fn new(id_cert: IdCert, resources: ResourceSet) -> Self {
        ChildDetails {
            id_cert,
            resources,
            used_keys: HashMap::new(),
        }
    }

    pub fn id_cert(&self) -> &IdCert {
        &self.id_cert
    }

    pub fn set_id_cert(&mut self, id_cert: IdCert) {
        self.id_cert = id_cert;
    }

    pub fn resources(&self) -> &ResourceSet {
        &self.resources
    }

    pub fn set_resources(&mut self, resources: ResourceSet) {
        self.resources = resources;
    }

    pub fn issued(&self, rcn: &ResourceClassName) -> Vec<KeyIdentifier> {
        let mut res = vec![];

        for (ki, last_response) in self.used_keys.iter() {
            if let LastResponse::Current(found_rcn) = last_response {
                if found_rcn == rcn {
                    res.push(*ki)
                }
            }
        }

        res
    }

    pub fn is_issued(&self, ki: &KeyIdentifier) -> bool {
        matches!(self.used_keys.get(ki), Some(LastResponse::Current(_)))
    }

    pub fn add_issue_response(&mut self, rcn: ResourceClassName, ki: KeyIdentifier) {
        self.used_keys.insert(ki, LastResponse::Current(rcn));
    }

    pub fn add_revoke_response(&mut self, ki: KeyIdentifier) {
        self.used_keys.insert(ki, LastResponse::Revoked);
    }

    /// Returns an error in case the key is already in use in another class.
    pub fn verify_key_allowed(&self, ki: &KeyIdentifier, rcn: &ResourceClassName) -> KrillResult<()> {
        if let Some(last_response) = self.used_keys.get(ki) {
            let allowed = match last_response {
                LastResponse::Revoked => false,
                LastResponse::Current(found) => found == rcn,
            };
            if !allowed {
                return Err(Error::KeyUseAttemptReuse);
            }
        }
        Ok(())
    }
}

impl From<ChildDetails> for ChildCaInfo {
    fn from(details: ChildDetails) -> Self {
        ChildCaInfo::new((&details.id_cert).into(), details.resources)
    }
}

//------------ Children ----------------------------------------------------

/// The collection of children under a parent [`CertAuth`].
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Children {
    inner: HashMap<ChildHandle, ChildDetails>,
}

//------------ ChildCertificates -------------------------------------------

/// The collection of certificates issued under a [ResourceClass](ca.ResourceClass).
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ChildCertificates {
    inner: HashMap<KeyIdentifier, IssuedCert>,
}

impl ChildCertificates {
    pub fn certificate_issued(&mut self, issued: IssuedCert) {
        self.inner.insert(issued.cert().subject_key_identifier(), issued);
    }

    pub fn key_revoked(&mut self, key: &KeyIdentifier) {
        self.inner.remove(key);
    }

    pub fn get(&self, ki: &KeyIdentifier) -> Option<&IssuedCert> {
        self.inner.get(ki)
    }

    pub fn current(&self) -> impl Iterator<Item = &IssuedCert> {
        self.inner.values()
    }

    pub fn expiring(&self, issuance_timing: &IssuanceTimingConfig) -> Vec<&IssuedCert> {
        self.inner
            .values()
            .filter(|issued| {
                issued.validity().not_after()
                    < Time::now() + Duration::weeks(issuance_timing.timing_child_certificate_reissue_weeks_before)
            })
            .collect()
    }

    pub fn overclaiming(&self, resources: &ResourceSet) -> Vec<&IssuedCert> {
        self.inner
            .values()
            .filter(|issued| !resources.contains(issued.resource_set()))
            .collect()
    }

    pub fn iter(&self) -> impl Iterator<Item = &IssuedCert> {
        self.inner.values()
    }
}

impl Default for ChildCertificates {
    fn default() -> Self {
        ChildCertificates { inner: HashMap::new() }
    }
}
