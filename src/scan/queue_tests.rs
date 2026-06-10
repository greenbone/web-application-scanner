// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::ScanQueue;

#[tokio::test]
async fn queue_is_fifo() {
    let queue = ScanQueue::new();
    queue.enqueue("scan-1".to_string()).await;
    queue.enqueue("scan-2".to_string()).await;

    assert_eq!(queue.dequeue().await, "scan-1");
    assert_eq!(queue.dequeue().await, "scan-2");
}

#[tokio::test]
async fn remove_deletes_requested_scan() {
    let queue = ScanQueue::new();
    queue.enqueue("scan-1".to_string()).await;
    queue.enqueue("scan-2".to_string()).await;

    assert!(queue.remove("scan-1").await);
    assert_eq!(queue.dequeue().await, "scan-2");
    assert!(!queue.remove("scan-3").await);
}
