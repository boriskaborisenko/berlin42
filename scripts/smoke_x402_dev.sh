#!/usr/bin/env bash
set -euo pipefail

API_BASE="${API_BASE:-http://127.0.0.1:8080}"

echo "1) Health"
curl -sS "$API_BASE/health"
echo
echo

echo "2) Payment requirements"
curl -sS "$API_BASE/api/payment/requirements"
echo
echo

echo "3) Unpaid live run should return HTTP 402"
curl -sS -i "$API_BASE/api/runs" \
  -H 'Content-Type: application/json' \
  -d '{"intent":"какой самый умный кот?","artifact_type":"answer","variant_count":1,"research_enabled":false,"execution_mode":"live"}' \
  | sed -n '1,18p'
echo

echo "4) Dev-paid live run should start when X402_DEV_BYPASS=true"
curl -sS "$API_BASE/api/runs" \
  -H 'Content-Type: application/json' \
  -H 'X402-DEV-PAYMENT: dev-paid' \
  -d '{"intent":"какой самый умный кот?","artifact_type":"answer","variant_count":1,"research_enabled":false,"execution_mode":"live"}'
echo
