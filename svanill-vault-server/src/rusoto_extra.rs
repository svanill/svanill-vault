use chrono::Datelike;
use chrono::{DateTime, Utc};
use rusoto_core::Region;
use rusoto_signature::signature::SignedRequest;
use serde::ser::{Serialize, SerializeSeq, Serializer};
use std::collections::HashMap;
use time::Date;

#[derive(Default)]
pub struct PostPolicy<'a> {
    expiration: Option<DateTime<Utc>>,
    content_length_range: Option<(u64, u64)>,
    conditions: Vec<Condition<'a>>,
    form_data: HashMap<String, String>,
    bucket_name: Option<&'a str>,
    key: Option<&'a str>,
    region: Option<&'a Region>,
    access_key_id: Option<&'a str>,
    secret_access_key: Option<&'a str>,
}

#[derive(Serialize)]
pub struct SerializablePolicy<'a> {
    expiration: &'a str,
    conditions: &'a Vec<Condition<'a>>,
}

struct Condition<'a>((&'a str, &'a str, &'a str));

impl<'a> Serialize for Condition<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(3))?;
        let v = &self.0;
        seq.serialize_element(v.0)?;

        if v.0 == "content-length-range" {
            seq.serialize_element(&v.1.parse::<u64>().map_err(|_| {
                serde::ser::Error::custom("expected u64 value, the minimum content length")
            })?)?;
            seq.serialize_element(&v.2.parse::<u64>().map_err(|_| {
                serde::ser::Error::custom("expected u64 value, the maximum content length")
            })?)?;
        } else {
            seq.serialize_element(v.1)?;
            seq.serialize_element(v.2)?;
        }

        seq.end()
    }
}

impl<'a> PostPolicy<'a> {
    /// Set expiration time
    pub fn set_expiration(mut self, t: DateTime<Utc>) -> Self {
        self.expiration = Some(t);
        self
    }

    /// Set key policy condition
    pub fn set_key(mut self, key: &'a str) -> Self {
        if key.is_empty() {
            return self;
        }

        self = self.append_policy("eq", "$key", &key);
        self.key = Some(key);
        self.form_data.insert("key".to_string(), key.to_string());
        self
    }

    /// Set key startswith policy condition
    #[allow(dead_code)]
    pub fn set_key_startswith(mut self, key_startswith: &'a str) -> Self {
        if key_startswith.is_empty() {
            return self;
        }

        self = self.append_policy("starts-with", "$key", &key_startswith);
        self.form_data
            .insert("key".to_string(), key_startswith.to_string());
        self
    }

    /// Set bucket name
    pub fn set_bucket_name(mut self, bucket_name: &'a str) -> Self {
        self.form_data
            .insert("bucket".to_string(), bucket_name.to_string());
        self = self.append_policy("eq", "$bucket", bucket_name);
        self.bucket_name = Some(bucket_name);
        self
    }

    /// Set region
    pub fn set_region(mut self, region: &'a Region) -> Self {
        self.region = Some(region);
        self
    }

    /// Set access key id
    pub fn set_access_key_id(mut self, access_key_id: &'a str) -> Self {
        if access_key_id.is_empty() {
            return self;
        }

        self.access_key_id = Some(access_key_id);
        self
    }

    /// Set secret access key
    pub fn set_secret_access_key(mut self, secret_access_key: &'a str) -> Self {
        if secret_access_key.is_empty() {
            return self;
        }

        self.secret_access_key = Some(secret_access_key);
        self
    }

    /// Set content-type policy condition
    #[allow(dead_code)]
    pub fn set_content_type(mut self, content_type: &'a str) -> Self {
        self.form_data
            .insert("Content-Type".to_string(), content_type.to_string());
        self = self.append_policy("eq", "$Content-Type", content_type);
        self
    }

    /// Set content length range policy condition
    pub fn set_content_length_range(mut self, min_length: u64, max_length: u64) -> Self {
        self.content_length_range = Some((min_length, max_length));
        // We should append the policy here, but ownership it's tricky,
        // so we'll do it inside build_form_data()
        self
    }

    /// Append policy condition
    pub fn append_policy(mut self, condition: &'a str, target: &'a str, value: &'a str) -> Self {
        self.conditions.push(Condition((condition, target, value)));
        self
    }

    /// Create the form data using the policy
    pub fn build_form_data(mut self) -> Result<(String, HashMap<String, String>), String> {
        match self.content_length_range {
            Some((min_length, max_length)) if min_length > max_length => {
                return Err(format!(
                    "Min-length ({}) must be <= Max-length ({})",
                    min_length, max_length
                ));
            }
            _ => (),
        }

        if self.expiration.is_none() {
            return Err("Expiration date must be specified".to_string());
        }

        if self.key.is_none() {
            return Err("Object key must be specified".to_string());
        }

        if self.bucket_name.is_none() {
            return Err("Bucket name must be specified".to_string());
        }

        if self.region.is_none() {
            return Err("Region must be specified".to_string());
        }

        if self.access_key_id.is_none() {
            return Err("Access key id must be specified".to_string());
        }

        if self.secret_access_key.is_none() {
            return Err("Secret access key must be specified".to_string());
        }

        let bucket_name = self.bucket_name.unwrap();
        let secret_access_key = self.secret_access_key.unwrap();

        let expiration = self
            .expiration
            .unwrap()
            .format("%Y-%m-%dT%H:%M:%S.000Z")
            .to_string();

        let current_time = Utc::now();
        let current_time_fmted = current_time.format("%Y%m%dT%H%M%SZ").to_string();
        let current_date = current_time.format("%Y%m%d").to_string();

        let access_key_id = self.access_key_id.unwrap();
        let region = self.region.unwrap();
        let region_name = region.name();

        let x_amz_credential = format!(
            "{}/{}/{}/{}/aws4_request",
            &access_key_id, &current_date, &region_name, "s3",
        );

        let mut conditions: Vec<Condition> = self.conditions.into_iter().collect();

        conditions.push(Condition(("eq", "$x-amz-date", &current_time_fmted)));
        conditions.push(Condition(("eq", "$x-amz-algorithm", "AWS4-HMAC-SHA256")));
        conditions.push(Condition(("eq", "$x-amz-credential", &x_amz_credential)));

        let min_length_as_string: String;
        let max_length_as_string: String;
        if let Some((min, max)) = self.content_length_range {
            min_length_as_string = min.to_string();
            max_length_as_string = max.to_string();
            conditions.push(Condition((
                "content-length-range",
                &min_length_as_string,
                &max_length_as_string,
            )))
        }

        let policy_to_serialize = SerializablePolicy {
            expiration: &expiration,
            conditions: &conditions,
        };

        let policy_as_json =
            serde_json::to_string(&policy_to_serialize).map_err(|e| format!("{:?}", e))?;

        let policy_as_base64 = base64::encode(policy_as_json);

        let signature_date = Date::try_from_ymd(
            current_time.date().year() as i32,
            current_time.date().month() as u8,
            current_time.date().day() as u8,
        )
        .unwrap();

        let x_amz_signature = signature::sign_string(
            &policy_as_base64,
            &secret_access_key,
            signature_date,
            &region_name,
            "s3",
        );

        self.form_data
            .insert("policy".to_string(), policy_as_base64);
        self.form_data
            .insert("x-amz-date".to_string(), current_time_fmted);
        self.form_data.insert(
            "x-amz-algorithm".to_string(),
            "AWS4-HMAC-SHA256".to_string(),
        );
        self.form_data
            .insert("x-amz-credential".to_string(), x_amz_credential);
        self.form_data
            .insert("x-amz-signature".to_string(), x_amz_signature);

        let signed_request = SignedRequest::new("GET", "s3", &region, "/");

        let upload_url = format!(
            "{}://{}.{}",
            signed_request.scheme(),
            bucket_name,
            signed_request.hostname()
        );

        Ok((upload_url, self.form_data))
    }
}

// Copied from rusoto/signature/src/signature.rs
// because `sign_string` was not public and I wanted to
// implement generate_presigned_post_policy in a way that
// could be easily implemented in rusoto_signature
mod signature {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    use time::Date;

    #[inline]
    fn hmac(secret: &[u8], message: &[u8]) -> Hmac<Sha256> {
        let mut hmac = Hmac::<Sha256>::new_varkey(secret).expect("failed to create hmac");
        hmac.input(message);
        hmac
    }

    /// Takes a message and signs it using AWS secret, time, region keys and service keys.
    pub fn sign_string(
        string_to_sign: &str,
        secret: &str,
        date: Date,
        region: &str,
        service: &str,
    ) -> String {
        let date_str = date.format("%Y%m%d");
        let date_hmac = hmac(format!("AWS4{}", secret).as_bytes(), date_str.as_bytes())
            .result()
            .code();
        let region_hmac = hmac(date_hmac.as_ref(), region.as_bytes()).result().code();
        let service_hmac = hmac(region_hmac.as_ref(), service.as_bytes())
            .result()
            .code();
        let signing_hmac = hmac(service_hmac.as_ref(), b"aws4_request").result().code();
        hex::encode(
            hmac(signing_hmac.as_ref(), string_to_sign.as_bytes())
                .result()
                .code()
                .as_ref(),
        )
    }
}
