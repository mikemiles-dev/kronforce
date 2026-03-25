// Your script here
let resp = http_get("https://example.com/health");
print("Status: " + resp.status);
if resp.status != 200 {
    fail("Health check failed");
}