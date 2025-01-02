use aws_sigv4::sign::v4::{calculate_signature, generate_signing_key};
use aws_smithy_types::date_time::{DateTime, Format};
use aws_types::region::Region;
use base64::{engine::general_purpose, Engine as _};
use serde::ser::{Serialize, SerializeSeq, Serializer};
use std::{collections::HashMap, time::SystemTime};

// Policy explanation:
// http://docs.aws.amazon.com/AmazonS3/latest/API/sigv4-HTTPPOSTConstructPolicy.html

#[derive(Default)]
pub struct PostPolicy<'a> {
    expiration: Option<&'a DateTime>,
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

/// Formats a `DateTime` in `YYYYMMDD'T'HHMMSS'Z'` format.
fn format_date_time(time: DateTime) -> Result<String, String> {
    // this is dumb, but I don't want to add a dependency for it
    // and I can't use the private omonymous function from aws_sigv4

    // e.g. 2019-12-16T23:48:18.52Z
    let s_full = time.fmt(Format::DateTime).or(Err("Cannot format time"))?;
    let s = s_full.as_str();

    Ok(format!(
        "{}{}{}T{}{}{}Z",
        &s[0..4],
        &s[5..7],
        &s[8..10],
        &s[11..13],
        &s[14..16],
        &s[17..19]
    ))
}

struct Condition<'a>((&'a str, &'a str, &'a str));

impl Serialize for Condition<'_> {
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
    pub fn set_expiration(mut self, t: &'a DateTime) -> Self {
        self.expiration = Some(t);
        self
    }

    /// Set key policy condition
    pub fn set_key(mut self, key: &'a str) -> Self {
        if key.is_empty() {
            return self;
        }

        self = self.append_policy("eq", "$key", key);
        self.key = Some(key);
        self.form_data.insert("key".to_string(), key.to_string());
        self
    }

    /// Set key startswith policy condition
    #[allow(dead_code)]
    pub fn set_key_startswith(mut self, key: &'a str) -> Self {
        if key.is_empty() {
            return self;
        }

        self.key = Some(key);

        self = self.append_policy("starts-with", "$key", key);
        self.form_data.insert("key".to_string(), key.to_string());
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
    pub fn append_policy(mut self, match_type: &'a str, target: &'a str, value: &'a str) -> Self {
        self.conditions.push(Condition((match_type, target, value)));
        self
    }

    /// Create the form data using the policy
    pub fn build_form_data(mut self) -> Result<HashMap<String, String>, String> {
        match self.content_length_range {
            Some((min_length, max_length)) if min_length > max_length => {
                return Err(format!(
                    "Min-length ({min_length}) must be <= Max-length ({max_length})"
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

        let secret_access_key = self.secret_access_key.unwrap();

        let expiration = self
            .expiration
            .unwrap()
            .fmt(Format::DateTime)
            .or(Err("Cannot format expiration date"))?;

        let current_time = if cfg!(test) {
            DateTime::from_str("2020-01-01T00:00:00.13Z", Format::DateTime).unwrap()
        } else {
            DateTime::from(SystemTime::now())
        };

        let current_time_fmted = format_date_time(current_time)?;
        let current_date = &current_time_fmted[0..8];

        let access_key_id = self.access_key_id.unwrap();
        let region = self.region.unwrap();
        let region_name = region.as_ref();

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
            serde_json::to_string(&policy_to_serialize).map_err(|e| format!("{e:?}"))?;

        let policy_as_base64 = general_purpose::STANDARD.encode(policy_as_json);

        let signature_date = std::time::SystemTime::now();

        let signing_key =
            generate_signing_key(secret_access_key, signature_date, region_name, "s3");
        let x_amz_signature = calculate_signature(signing_key, policy_as_base64.as_bytes());

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

        Ok(self.form_data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BUCKET: &str = "the-bucket";
    const REGION: Region = Region::from_static("eu-central-1");
    const ACCESS_KEY_ID: &str = "foo_access_key";
    const SECRET_ACCESS_KEY: &str = "foo_secret_key";
    const OBJECT_KEY: &str = "the-object-key";

    #[test]
    fn bucket_name_is_required() {
        let expiration_date = DateTime::from_str("2020-01-01T01:02:03Z", Format::DateTime).unwrap();

        let res = PostPolicy::default()
            .set_region(&REGION)
            .set_access_key_id(ACCESS_KEY_ID)
            .set_secret_access_key(SECRET_ACCESS_KEY)
            .set_key(OBJECT_KEY)
            .set_expiration(&expiration_date)
            .build_form_data();

        assert_eq!(res, Err("Bucket name must be specified".to_string()));
    }

    #[test]
    fn region_is_required() {
        let expiration_date = DateTime::from_str("2020-01-01T01:02:03Z", Format::DateTime).unwrap();

        let res = PostPolicy::default()
            .set_bucket_name(BUCKET)
            .set_access_key_id(ACCESS_KEY_ID)
            .set_secret_access_key(SECRET_ACCESS_KEY)
            .set_key(OBJECT_KEY)
            .set_expiration(&expiration_date)
            .build_form_data();

        assert_eq!(res, Err("Region must be specified".to_string()));
    }
    #[test]
    fn access_key_id_is_required() {
        let expiration_date = DateTime::from_str("2020-01-01T01:02:03Z", Format::DateTime).unwrap();

        let res = PostPolicy::default()
            .set_bucket_name(BUCKET)
            .set_region(&REGION)
            .set_secret_access_key(SECRET_ACCESS_KEY)
            .set_key(OBJECT_KEY)
            .set_expiration(&expiration_date)
            .build_form_data();

        assert_eq!(res, Err("Access key id must be specified".to_string()));
    }

    #[test]
    fn secret_access_key_is_required() {
        let expiration_date = DateTime::from_str("2020-01-01T01:02:03Z", Format::DateTime).unwrap();

        let res = PostPolicy::default()
            .set_bucket_name(BUCKET)
            .set_region(&REGION)
            .set_access_key_id(ACCESS_KEY_ID)
            .set_key(OBJECT_KEY)
            .set_expiration(&expiration_date)
            .build_form_data();

        assert_eq!(res, Err("Secret access key must be specified".to_string()));
    }

    #[test]
    fn expiration_is_required() {
        let res = PostPolicy::default()
            .set_bucket_name(BUCKET)
            .set_region(&REGION)
            .set_access_key_id(ACCESS_KEY_ID)
            .set_key(OBJECT_KEY)
            .build_form_data();

        assert_eq!(res, Err("Expiration date must be specified".to_string()));
    }
    #[test]
    fn build_successfully() {
        let expiration_date = DateTime::from_str("2020-01-01T01:02:03Z", Format::DateTime).unwrap();

        let res = PostPolicy::default()
            .set_bucket_name(BUCKET)
            .set_region(&REGION)
            .set_access_key_id(ACCESS_KEY_ID)
            .set_secret_access_key(SECRET_ACCESS_KEY)
            .set_key(OBJECT_KEY)
            .set_expiration(&expiration_date)
            .set_content_length_range(123, 456)
            .build_form_data();

        assert!(res.is_ok());
        let form_data = res.unwrap();

        assert_eq!(form_data.get("key").unwrap(), "the-object-key");

        assert_eq!(form_data.get("bucket").unwrap(), "the-bucket");
        assert_eq!(
            form_data.get("x-amz-algorithm").unwrap(),
            "AWS4-HMAC-SHA256"
        );
        assert_eq!(
            form_data.get("x-amz-credential").unwrap(),
            "foo_access_key/20200101/eu-central-1/s3/aws4_request"
        );
        assert_eq!(form_data.get("x-amz-date").unwrap(), "20200101T000000Z");

        let expected_policy = serde_json::json!({
            "expiration": "2020-01-01T01:02:03Z",
            "conditions": [
                ["eq", "$bucket", "the-bucket"],
                ["eq", "$key", "the-object-key"],
                ["eq", "$x-amz-date", "20200101T000000Z"],
                ["eq", "$x-amz-algorithm", "AWS4-HMAC-SHA256"],
                ["eq", "$x-amz-credential", "foo_access_key/20200101/eu-central-1/s3/aws4_request"],
                ["content-length-range", 123, 456]
            ]
        });

        let policy_as_base64 = form_data.get("policy").unwrap();
        let policy_as_vec_u8 = general_purpose::STANDARD.decode(policy_as_base64).unwrap();
        let policy: serde_json::Value = serde_json::from_slice(&policy_as_vec_u8).unwrap();
        assert_eq!(policy, expected_policy);
    }

    #[test]
    fn set_content_type() {
        let expiration_date = DateTime::from_str("2020-01-01T01:02:03Z", Format::DateTime).unwrap();

        let res = PostPolicy::default()
            .set_content_type("some/type")
            .set_bucket_name(BUCKET)
            .set_region(&REGION)
            .set_access_key_id(ACCESS_KEY_ID)
            .set_secret_access_key(SECRET_ACCESS_KEY)
            .set_key(OBJECT_KEY)
            .set_expiration(&expiration_date)
            .build_form_data();

        assert!(res.is_ok());

        let form_data = res.unwrap();
        dbg!(&form_data);
        assert_eq!(form_data.get("Content-Type").unwrap(), "some/type");

        let policy_as_base64 = form_data.get("policy").unwrap();
        let policy_as_vec_u8 = general_purpose::STANDARD.decode(policy_as_base64).unwrap();
        let policy: serde_json::Value = serde_json::from_slice(&policy_as_vec_u8).unwrap();
        let conditions = policy["conditions"].as_array().unwrap();
        assert!(conditions.contains(&serde_json::json!(["eq", "$Content-Type", "some/type"])));
    }

    #[test]
    fn append_policy() {
        let expiration_date = DateTime::from_str("2020-01-01T01:02:03Z", Format::DateTime).unwrap();

        let res = PostPolicy::default()
            .append_policy("a", "b", "c")
            .set_bucket_name(BUCKET)
            .set_region(&REGION)
            .set_access_key_id(ACCESS_KEY_ID)
            .set_secret_access_key(SECRET_ACCESS_KEY)
            .set_key(OBJECT_KEY)
            .set_expiration(&expiration_date)
            .build_form_data();

        let form_data = res.unwrap();

        assert_eq!(form_data.get("a"), None);

        let policy_as_base64 = form_data.get("policy").unwrap();
        let policy_as_vec_u8 = general_purpose::STANDARD.decode(policy_as_base64).unwrap();
        let policy: serde_json::Value = serde_json::from_slice(&policy_as_vec_u8).unwrap();
        let conditions = policy["conditions"].as_array().unwrap();
        assert!(conditions.contains(&serde_json::json!(["a", "b", "c"])));
    }

    #[test]
    fn set_key_startswith() {
        let expiration_date = DateTime::from_str("2020-01-01T01:02:03Z", Format::DateTime).unwrap();

        let res = PostPolicy::default()
            .set_key_startswith("foo")
            .set_bucket_name(BUCKET)
            .set_region(&REGION)
            .set_access_key_id(ACCESS_KEY_ID)
            .set_secret_access_key(SECRET_ACCESS_KEY)
            .set_expiration(&expiration_date)
            .build_form_data();

        let form_data = res.unwrap();
        dbg!(&form_data);
        assert_eq!(form_data.get("key").unwrap(), "foo");

        let policy_as_base64 = form_data.get("policy").unwrap();
        let policy_as_vec_u8 = general_purpose::STANDARD.decode(policy_as_base64).unwrap();
        let policy: serde_json::Value = serde_json::from_slice(&policy_as_vec_u8).unwrap();
        let conditions = policy["conditions"].as_array().unwrap();
        assert!(conditions.contains(&serde_json::json!(["starts-with", "$key", "foo"])));
    }
}
