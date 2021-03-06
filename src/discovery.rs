use crate::AppState;
use actix_web::{web, Error, HttpResponse};
use biscuit::jwa;
use biscuit::jwa::Algorithm;
use biscuit::jwk::*;
use biscuit::jws::Secret;
use biscuit::Empty;
use num::BigUint;
use ring::signature::KeyPair;
use serde_json::json;

pub async fn keys(state: web::Data<AppState>) -> Result<HttpResponse, Error> {
    let rsa_key = &state.rsa_key_pair;

    let public_key = match rsa_key {
        Secret::RsaKeyPair(ring_pair) => {
            let s = ring_pair.clone();
            let pk = s.public_key().clone();
            Some(pk)
        }
        _ => None,
    }
    .expect("There is no RsaKeyPair with a public key found");

    let jwk_set: JWKSet<Empty> = JWKSet {
        keys: vec![JWK {
            common: CommonParameters {
                algorithm: Some(Algorithm::Signature(jwa::SignatureAlgorithm::RS256)),
                key_id: Some("2020-01-29".to_string()),
                ..Default::default()
            },
            algorithm: AlgorithmParameters::RSA(RSAKeyParameters {
                n: BigUint::from_bytes_be(public_key.modulus().big_endian_without_leading_zero()),
                e: BigUint::from_bytes_be(public_key.exponent().big_endian_without_leading_zero()),
                ..Default::default()
            }),
            additional: Default::default(),
        }],
    };

    Ok(HttpResponse::Ok().json(jwk_set))
}

pub async fn openid_configuration(_state: web::Data<AppState>) -> Result<HttpResponse, Error> {
    let keys_response = json!( {
      "issuer": "http://localhost:8080/",
      "authorization_endpoint": "http://localhost:8080/auth",
      "token_endpoint": "http://localhost:8080/token",
      "jwks_uri": "http://localhost:8080/keys",
      "userinfo_endpoint": "http://localhost:8080/userinfo",
      "response_types_supported": [
        "code",
        "id_token",
        "token"
      ],
      "subject_types_supported": [
        "public"
      ],
      "id_token_signing_alg_values_supported": [
        "RS256"
      ],
      "scopes_supported": [
        "openid",
        "email",
        "groups",
        "profile",
        "offline_access"
      ],
      "token_endpoint_auth_methods_supported": [
        "client_secret_basic"
      ],
      "claims_supported": [
        "aud",
        "email",
        "email_verified",
        "exp",
        "iat",
        "iss",
        "locale",
        "name",
        "sub"
      ]
    });
    Ok(HttpResponse::Ok().json(keys_response))
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::dev::Service;
    use actix_web::{http, test, web, App};
    use serde_json::json;
    use serde_json::Value;
    use std::str;

    #[actix_rt::test]
    async fn test_route_keys() -> Result<(), Error> {
        let mut app = test::init_service(
            App::new()
                .data(AppState::new("./static/private_key.der"))
                .service(web::resource("/").route(web::get().to(keys))),
        )
        .await;

        let req = test::TestRequest::get().uri("/").to_request();

        let resp = app.call(req).await.unwrap();

        assert_eq!(resp.status(), http::StatusCode::OK);

        let response_body = match resp.response().body().as_ref() {
            Some(actix_web::body::Body::Bytes(bytes)) => bytes,
            _ => panic!("Response error"),
        };

        let body_str = match str::from_utf8(&response_body) {
            Ok(v) => v,
            Err(_e) => "Error with parsing result from bytes to string",
        };

        let p: Value = serde_json::from_str(body_str).unwrap();
        println!("Value : {:?}", p);
        assert_eq!(p["keys"][0]["e"], json!("AQAB"));

        Ok(())
    }
}
