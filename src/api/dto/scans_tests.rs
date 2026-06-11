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
    let decoded =
        serde_json::from_str::<ScanRequest>(&json).expect("scan request should deserialize");

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
    let decoded =
        serde_json::from_str::<ScanActionRequest>(&json).expect("scan action should deserialize");

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

#[test]
fn scan_status_response_omits_host_info_when_none() {
    use crate::api::dto::scans::ScanStatusResponse;
    let response = ScanStatusResponse {
        status: ScanStatus::Stored,
        start_time: None,
        end_time: None,
        host_info: None,
    };
    let json = serde_json::to_value(&response).expect("should serialize");
    assert!(!json.as_object().unwrap().contains_key("host_info"));
}

#[test]
fn scan_status_response_includes_host_info_when_present() {
    use crate::api::dto::scans::{HostInfo, HostScanningEntry, ScanStatusResponse};
    let response = ScanStatusResponse {
        status: ScanStatus::Running,
        start_time: Some(1000),
        end_time: None,
        host_info: Some(HostInfo {
            all: 2,
            excluded: 0,
            dead: 0,
            alive: 2,
            queued: 1,
            finished: 0,
            scanning: vec![HostScanningEntry {
                host: "http://a.example".to_string(),
                progress: 1,
            }],
        }),
    };
    let json = serde_json::to_value(&response).expect("should serialize");
    assert_eq!(json["host_info"]["all"], 2);
    assert_eq!(json["host_info"]["alive"], 2);
    assert_eq!(json["host_info"]["queued"], 1);
    assert_eq!(json["host_info"]["scanning"][0]["host"], "http://a.example");
    assert_eq!(json["host_info"]["scanning"][0]["progress"], 1);
}

#[test]
fn host_info_round_trips_with_serde_json() {
    use crate::api::dto::scans::{HostInfo, HostScanningEntry};
    let info = HostInfo {
        all: 3,
        excluded: 0,
        dead: 0,
        alive: 3,
        queued: 1,
        finished: 1,
        scanning: vec![HostScanningEntry {
            host: "http://b.example".to_string(),
            progress: 62,
        }],
    };
    let json = serde_json::to_string(&info).expect("host info should serialize");
    let decoded = serde_json::from_str::<HostInfo>(&json).expect("host info should deserialize");
    assert_eq!(decoded, info);
}
