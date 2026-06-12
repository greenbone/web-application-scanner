#! /usr/bin/env python3

# SPDX-FileCopyrightText: 2026 Greenbone AG
#
# SPDX-License-Identifier: AGPL-3.0-or-later


# This script is meant to be used as a quick way to test if the WAS server is working correctly
#  and can run a scan.
# It will create a scan with the specified target, start it, wait for it to complete,
#  and then print the results.
# It can also write the results to a file if specified.
#

import argparse
import os
import time

# This script requires the httpx library.
import httpx

def main():
    parser = argparse.ArgumentParser(description="Try running a test scan using the WAS.")
    parser.add_argument("target", help="The host of the WAS server")
    parser.add_argument("--report-out", help="The file to write the report to (default: report.json)", default="report.json")
    parser.add_argument("--keep-scan", help="Whether to keep the scan after completion (default: False)", action="store_true")
    was_port = int(os.getenv("GREENBONE_WAS_PORT", "8030"))

    args = parser.parse_args()
    test_scan(was_port, target=args.target, report_out=args.report_out, keep_scan=args.keep_scan)

def test_scan(was_port: int, target: str, report_out: str = "report.json", keep_scan: bool = False):
    print (f"Using port {was_port} for the WAS server.")
    print (f"Trying to run a test scan against '{target}'.")

    url_prefix = f"http://localhost:{was_port}/api/v1"
    with httpx.Client() as client:
        # Check if the WAS server is healthy before trying to create a scan
        response = client.head(f"{url_prefix}/health")
        if not response.is_success:
            print ("WAS server is not healthy.")
            print (f"Response status code: {response.status_code}")
            print (f"Response content: {response.text}")
            return

        # Create a scan with the specified target
        scan_request = {
            "target": {
                "hosts": [target],
            },
            "vts": [],
        }
        print (f"Creating a scan with the following request body: {scan_request}")
        response = client.post(f"{url_prefix}/scans", json={"target": {"hosts": [target]}, "vts": []})
        if not response.is_success:
            print ("Failed to create the scan.")
            print (f"Response status code: {response.status_code}")
            print (f"Response content: {response.text}")
            return
        scan_id = response.json()
        print (f"Scan creation response: {scan_id}")
        print (f"Scan created with ID: {scan_id}")

        # Start the scan
        response = client.post(f"{url_prefix}/scans/{scan_id}", json={"action": "start"})
        if not response.is_success:
            print ("Failed to start the scan.")
            print (f"Response status code: {response.status_code}")
            print (f"Response content: {response.text}")
            return
        print ("scan started successfully.")

        done = False
        while not done:
            response = client.get(f"{url_prefix}/scans/{scan_id}/status")
            if not response.is_success:
                print ("Failed to get the scan status.")
                print (f"Response status code: {response.status_code}")
                print (f"Response content: {response.text}")
                return
            scan_status = response.json().get("status")
            print (f"Current scan status: {scan_status} - {response.json()}")
            if scan_status in ["succeeded", "stopped", "failed"]:
                done = True
                break
            time.sleep(5)

        print ("scan completed.")
        
        response = client.get(f"{url_prefix}/scans/{scan_id}/results")
        if not response.is_success:
            print ("Failed to get the scan results.")
            print (f"Response status code: {response.status_code}")
            print (f"Response content: {response.text}")
            return
        print (f"Received {len(response.json())} results")
        if (report_out):
            with open(report_out, "w") as f:
                f.write(response.text)
            print (f"Scan results written to '{report_out}'.")

        if not keep_scan:
            response = client.delete(f"{url_prefix}/scans/{scan_id}")
            if not response.is_success:
                print ("Failed to delete the scan.")
                print (f"Response status code: {response.status_code}")
                print (f"Response content: {response.text}")
                return
            print (f"Scan {scan_id} deleted successfully.")
        else:
            print (f"Scan with ID {scan_id} kept.")

if __name__ == "__main__":
    main ()