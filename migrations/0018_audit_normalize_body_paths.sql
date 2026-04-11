-- Audit body paths must be relative to [audit].log_dir only (e.g. YYYYMMDD/<request_id>-request.json).
-- Older buggy rows may contain a duplicated path prefix ending in /audit_logs/ (e.g. ../../shared/audit_logs/...).

UPDATE audit_logs
SET request_body_path = CASE
    WHEN request_body_path IS NOT NULL AND instr(request_body_path, '/audit_logs/') > 0
    THEN substr(
        request_body_path,
        instr(request_body_path, '/audit_logs/') + length('/audit_logs/')
    )
    ELSE request_body_path
END
WHERE request_body_path IS NOT NULL;

UPDATE audit_logs
SET response_body_path = CASE
    WHEN response_body_path IS NOT NULL AND instr(response_body_path, '/audit_logs/') > 0
    THEN substr(
        response_body_path,
        instr(response_body_path, '/audit_logs/') + length('/audit_logs/')
    )
    ELSE response_body_path
END
WHERE response_body_path IS NOT NULL;
