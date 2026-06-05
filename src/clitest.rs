// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::thread::sleep;
use std::time::Duration;

const API_KEY: &str = "test-api-key";
const BASE_URL: &str = "http://localhost:8080";

use greenbone_was::zapclient::{ZapClient, ajaxspider::AjaxSpiderStatus};

#[tokio::main]
async fn main() {
    println!(
        "Testing ZAP client with base URL: {} and API key: {}",
        BASE_URL, API_KEY
    );

    let target_url = std::env::var("TARGET_URL").unwrap();
    println!("Using target URL: {}", target_url);

    let target_regex = regex::escape(&target_url) + ".*";

    let zap_client = ZapClient::new(BASE_URL.to_string(), API_KEY.to_string())
        .expect("Failed to create ZAP client");

    let context_name = ("greenbone-".to_string() + &uuid::Uuid::new_v4().to_string()).to_string();

    let context_id = zap_client
        .new_context(&context_name)
        .await
        .expect("Failed to create context");
    println!(
        "Created context '{}' with ID '{}'",
        context_name, context_id
    );

    let contexts = zap_client
        .get_context_list()
        .await
        .expect("Failed to get context list");
    println!("Available contexts: {:?}", contexts);

    zap_client
        .include_in_context(&context_name, &target_regex)
        .await
        .expect("Failed to include in context");
    println!(
        "Included regex '{}' in context '{}'",
        target_regex, context_name
    );

    zap_client
        .start_ajax_spider_scan(&context_name, &target_url, true, true)
        .await
        .expect("Failed to start AJAX Spider scan");
    println!(
        "Started AJAX Spider scan for context '{}' and target URL '{}'",
        context_name, target_url
    );

    sleep(Duration::from_secs(5));
    println!("Waited for 5 seconds to let the scan start...");

    let mut status = zap_client
        .get_ajax_spider_status()
        .await
        .expect("Failed to get AJAX Spider status");
    while status != AjaxSpiderStatus::Stopped {
        println!("AJAX Spider scan is still running... Status: {:?}", status);
        sleep(Duration::from_secs(5));
        status = zap_client
            .get_ajax_spider_status()
            .await
            .expect("Failed to get AJAX Spider status");
    }
    println!("AJAX Spider scan completed with status: {:?}", status);

    sleep(Duration::from_secs(5));
    println!("Waited for 5 seconds before active scan start...");

    let scan_id = zap_client
        .start_active_scan(&context_id, &target_url, true, true)
        .await
        .expect("Failed to start active scan");
    println!(
        "Started active scan for context id '{}' and target URL '{}'",
        context_id, target_url
    );

    let mut active_scan_status = -1;

    while active_scan_status < 100 {
        active_scan_status = zap_client
            .get_active_scan_status(&scan_id)
            .await
            .expect("Failed to get active scan status");
        println!(
            "Active scan is still running... Status: {}%",
            active_scan_status
        );
        sleep(Duration::from_secs(10));
    }

    println!("Active scan completed with status: {}%", active_scan_status);

    //    let context_name = "greenbone-6c689dd8-db1b-4eaf-ab09-facdb98adf71".to_string();

    let alerts = zap_client
        .get_alerts(&context_name, None, None, None)
        .await
        .expect("Failed to get alerts");
    println!(
        "Retrieved {} alerts for context '{}'",
        alerts.len(),
        context_name
    );

    for alert in alerts {
        println!(
            "Alert [{:?}]: {} - {} ({}:{})",
            alert.risk, alert.name, alert.description, alert.url, alert.plugin_id
        );
    }

    zap_client
        .remove_context(&context_name)
        .await
        .expect("Failed to remove context");

    let contexts = zap_client
        .get_context_list()
        .await
        .expect("Failed to get context list");
    println!("Available contexts after removal: {:?}", contexts);
}
