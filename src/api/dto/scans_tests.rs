// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::{
    Credential, Parameter, ScanAction, ScanActionRequest, ScanDetailResponse, ScanRequest,
    ScanResultResponse, ScanStatusResponse, ScannerPreference, Target, UsernamePasswordCredential,
    Vt,
};
use crate::{api::dto::scans::ResultType, scan::ScanStatus};

#[test]
fn scan_request_serializes_and_deserializes_with_serde_json() {
    let payload = ScanRequest {
        target: Target {
            hosts: vec!["https://example.com".to_string()],
            excluded_hosts: vec!["https://example.com/logout".to_string()],
            credentials: vec![Credential {
                service: "http".to_string(),
                port: Some(443),
                up: Some(UsernamePasswordCredential {
                    username: "alice".to_string(),
                    password: Some("secret".to_string()),
                    privilege_username: None,
                    privilege_password: None,
                }),
            }],
        },
        scan_preferences: vec![ScannerPreference {
            id: "request-timeout".to_string(),
            value: "10".to_string(),
        }],
        vts: vec![Vt {
            oid: "1.3.6.1.4.1.25623.1.0.100000".to_string(),
            parameters: vec![Parameter {
                id: 1,
                value: "enabled".to_string(),
            }],
        }],
    };

    let json = serde_json::to_string(&payload).expect("scan request should serialize");
    let decoded = serde_json::from_str::<ScanRequest>(&json)
        .expect("scan request should deserialize");

    assert_eq!(decoded, payload);
}

#[test]
fn scan_request_round_trips_with_empty_collections() {
    let json = r#"{
        "target": {
            "hosts": ["https://example.com"],
            "excluded_hosts": [],
            "credentials": []
        },
        "scan_preferences": [],
        "vts": []
    }"#;

    let decoded = serde_json::from_str::<ScanRequest>(json)
        .expect("scan request with empty collections should deserialize");

    assert_eq!(
        decoded,
        ScanRequest {
            target: Target {
                hosts: vec!["https://example.com".to_string()],
                excluded_hosts: vec![],
                credentials: vec![],
            },
            scan_preferences: vec![],
            vts: vec![],
        }
    );

    let encoded = serde_json::to_value(&decoded)
        .expect("scan request with empty collections should serialize");

    assert_eq!(
        encoded,
        serde_json::json!({
            "target": {
                "hosts": ["https://example.com"],
                "excluded_hosts": [],
                "credentials": [],
            },
            "scan_preferences": [],
            "vts": [],
        })
    );
}

#[test]
fn scan_action_request_round_trips_with_lowercase_action() {
    let payload = ScanActionRequest {
        action: ScanAction::Start,
    };

    let json = serde_json::to_string(&payload).expect("scan action should serialize");
    let decoded = serde_json::from_str::<ScanActionRequest>(&json)
        .expect("scan action should deserialize");

    assert_eq!(json, r#"{"action":"start"}"#);
    assert_eq!(decoded, payload);
}

#[test]
fn scan_response_payloads_deserialize_with_serde_json() {
    let detail = serde_json::from_str::<ScanDetailResponse>(
        r#"{
            "scan_id": "scan-123",
            "target": {
                "hosts": ["https://example.com"],
                "excluded_hosts": [],
                "credentials": []
            },
            "scan_preferences": [{"id": "depth", "value": "3"}],
            "vts": [{"oid": "1.3.6.1.4.1.25623.1.0.100000", "parameters": []}]
        }"#,
    )
    .expect("scan detail response should deserialize");
    assert_eq!(detail.scan_id, "scan-123");

    let status = serde_json::from_str::<ScanStatusResponse>(
        r#"{"status":"running","start_time":123,"end_time":null}"#,
    )
    .expect("scan status response should deserialize");
    assert_eq!(status.status, ScanStatus::Running);
    assert_eq!(status.start_time, Some(123));
    assert_eq!(status.end_time, None);

    let result = serde_json::from_str::<ScanResultResponse>(
        r#"{
            "id": 7,
            "type": "alarm",
            "ip_address": "192.0.2.10",
            "message": "XSS detected",
            "detail": {"risk": "high"}
        }"#,
    )
    .expect("scan result response should deserialize");
    assert_eq!(result.id, 7);
    assert_eq!(result.result_type, ResultType::Alarm);
    assert_eq!(result.ip_address.as_deref(), Some("192.0.2.10"));
    assert_eq!(result.message.as_deref(), Some("XSS detected"));
    assert_eq!(result.detail, Some(serde_json::json!({"risk": "high"})));
}