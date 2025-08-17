# Monitoring and Observability Guide

## Overview

This guide covers setting up comprehensive monitoring for Readur, including metrics collection, log aggregation, alerting, and dashboard creation.

## Monitoring Stack Components

### Core Components

1. **Metrics Collection**: Prometheus + Node Exporter
2. **Visualization**: Grafana
3. **Log Aggregation**: Loki or ELK Stack
4. **Alerting**: AlertManager
5. **Application Monitoring**: Custom metrics and health checks
6. **Uptime Monitoring**: Uptime Kuma or Pingdom

## Health Monitoring

### Built-in Health Endpoints

```bash
# Basic health check
curl http://localhost:8000/health

# Detailed health status
curl http://localhost:8000/health/detailed

# Response format
{
  "status": "healthy",
  "database": "connected",
  "redis": "connected",
  "storage": "accessible",
  "ocr_queue": 45,
  "version": "2.5.4",
  "uptime": 345600
}
```

### Custom Health Checks

```python
# health_checks.py
from typing import Dict, Any

class HealthMonitor:
    @staticmethod
    def check_database() -> Dict[str, Any]:
        try:
            db.session.execute("SELECT 1")
            return {"status": "healthy", "response_time": 0.005}
        except Exception as e:
            return {"status": "unhealthy", "error": str(e)}
    
    @staticmethod
    def check_storage() -> Dict[str, Any]:
        try:
            # Check if storage is accessible
            storage.list_files(limit=1)
            return {"status": "healthy", "available_space": storage.get_free_space()}
        except Exception as e:
            return {"status": "unhealthy", "error": str(e)}
    
    @staticmethod
    def check_ocr_workers() -> Dict[str, Any]:
        active = celery.control.inspect().active()
        return {
            "status": "healthy" if active else "degraded",
            "active_workers": len(active or {}),
            "queue_length": redis.llen("ocr_queue")
        }
```

## Prometheus Setup

### Installation and Configuration

```yaml
# docker-compose.monitoring.yml
version: '3.8'

services:
  prometheus:
    image: prom/prometheus:latest
    container_name: prometheus
    volumes:
      - ./prometheus/prometheus.yml:/etc/prometheus/prometheus.yml
      - prometheus_data:/prometheus
    ports:
      - "9090:9090"
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.path=/prometheus'
      - '--storage.tsdb.retention.time=30d'
    networks:
      - monitoring

  node-exporter:
    image: prom/node-exporter:latest
    container_name: node-exporter
    volumes:
      - /proc:/host/proc:ro
      - /sys:/host/sys:ro
      - /:/rootfs:ro
    command:
      - '--path.procfs=/host/proc'
      - '--path.sysfs=/host/sys'
      - '--collector.filesystem.mount-points-exclude=^/(sys|proc|dev|host|etc)($$|/)'
    ports:
      - "9100:9100"
    networks:
      - monitoring

  postgres-exporter:
    image: prometheuscommunity/postgres-exporter:latest
    container_name: postgres-exporter
    environment:
      DATA_SOURCE_NAME: "postgresql://readur:password@postgres:5432/readur?sslmode=disable"
    ports:
      - "9187:9187"
    networks:
      - monitoring

  redis-exporter:
    image: oliver006/redis_exporter:latest
    container_name: redis-exporter
    environment:
      REDIS_ADDR: "redis://redis:6379"
    ports:
      - "9121:9121"
    networks:
      - monitoring

networks:
  monitoring:
    external: true

volumes:
  prometheus_data:
```

### Prometheus Configuration

```yaml
# prometheus/prometheus.yml
global:
  scrape_interval: 15s
  evaluation_interval: 15s
  external_labels:
    monitor: 'readur-monitor'

alerting:
  alertmanagers:
    - static_configs:
        - targets:
            - alertmanager:9093

rule_files:
  - '/etc/prometheus/alerts/*.yml'

scrape_configs:
  - job_name: 'readur'
    static_configs:
      - targets: ['readur:8000']
    metrics_path: '/metrics'

  - job_name: 'node'
    static_configs:
      - targets: ['node-exporter:9100']

  - job_name: 'postgres'
    static_configs:
      - targets: ['postgres-exporter:9187']

  - job_name: 'redis'
    static_configs:
      - targets: ['redis-exporter:9121']
```

## Grafana Dashboards

### Setup Grafana

```yaml
# Add to docker-compose.monitoring.yml
grafana:
  image: grafana/grafana:latest
  container_name: grafana
  environment:
    - GF_SECURITY_ADMIN_USER=admin
    - GF_SECURITY_ADMIN_PASSWORD=changeme
    - GF_SERVER_ROOT_URL=https://grafana.readur.company.com
    - GF_INSTALL_PLUGINS=redis-datasource
  volumes:
    - grafana_data:/var/lib/grafana
    - ./grafana/provisioning:/etc/grafana/provisioning
  ports:
    - "3000:3000"
  networks:
    - monitoring
```

### Dashboard Configuration

```json
# grafana/provisioning/dashboards/readur.json
{
  "dashboard": {
    "title": "Readur Performance Dashboard",
    "panels": [
      {
        "title": "Request Rate",
        "targets": [{
          "expr": "rate(readur_requests_total[5m])"
        }]
      },
      {
        "title": "Response Time",
        "targets": [{
          "expr": "histogram_quantile(0.95, rate(readur_request_duration_seconds_bucket[5m]))"
        }]
      },
      {
        "title": "OCR Queue",
        "targets": [{
          "expr": "readur_ocr_queue_length"
        }]
      },
      {
        "title": "Database Connections",
        "targets": [{
          "expr": "pg_stat_database_numbackends{datname='readur'}"
        }]
      }
    ]
  }
}
```

## Application Metrics

### Custom Metrics Implementation

```python
# metrics.py
from prometheus_client import Counter, Histogram, Gauge, generate_latest

# Define metrics
request_count = Counter('readur_requests_total', 'Total requests', ['method', 'endpoint'])
request_duration = Histogram('readur_request_duration_seconds', 'Request duration')
ocr_queue_length = Gauge('readur_ocr_queue_length', 'OCR queue length')
active_users = Gauge('readur_active_users', 'Active users in last 5 minutes')
document_count = Gauge('readur_documents_total', 'Total documents', ['status'])

# Middleware to track requests
class MetricsMiddleware:
    def __init__(self, app):
        self.app = app
    
    def __call__(self, environ, start_response):
        path = environ.get('PATH_INFO', '/')
        method = environ.get('REQUEST_METHOD', 'GET')
        
        with request_duration.time():
            request_count.labels(method=method, endpoint=path).inc()
            return self.app(environ, start_response)

# Metrics endpoint
@app.route('/metrics')
def metrics():
    # Update gauges
    ocr_queue_length.set(redis.llen('ocr_queue'))
    active_users.set(get_active_user_count())
    document_count.labels(status='processed').set(get_document_count('processed'))
    
    return generate_latest(), 200, {'Content-Type': 'text/plain'}
```

## Log Aggregation

### Loki Setup

```yaml
# Add to docker-compose.monitoring.yml
loki:
  image: grafana/loki:latest
  container_name: loki
  ports:
    - "3100:3100"
  volumes:
    - ./loki/loki-config.yml:/etc/loki/loki-config.yml
    - loki_data:/loki
  command: -config.file=/etc/loki/loki-config.yml
  networks:
    - monitoring

promtail:
  image: grafana/promtail:latest
  container_name: promtail
  volumes:
    - /var/log:/var/log:ro
    - /var/lib/docker/containers:/var/lib/docker/containers:ro
    - ./promtail/promtail-config.yml:/etc/promtail/promtail-config.yml
  command: -config.file=/etc/promtail/promtail-config.yml
  networks:
    - monitoring
```

### Log Configuration

```yaml
# promtail/promtail-config.yml
server:
  http_listen_port: 9080
  grpc_listen_port: 0

positions:
  filename: /tmp/positions.yaml

clients:
  - url: http://loki:3100/loki/api/v1/push

scrape_configs:
  - job_name: readur
    docker_sd_configs:
      - host: unix:///var/run/docker.sock
        refresh_interval: 5s
        filters:
          - name: label
            values: ["com.docker.compose.project=readur"]
    relabel_configs:
      - source_labels: ['__meta_docker_container_name']
        regex: '/(.*)'
        target_label: 'container'
      - source_labels: ['__meta_docker_container_log_stream']
        target_label: 'logstream'
```

## Alerting

### AlertManager Configuration

```yaml
# alertmanager/config.yml
global:
  smtp_from: 'alertmanager@readur.company.com'
  smtp_smarthost: 'smtp.company.com:587'
  smtp_auth_username: 'alertmanager@readur.company.com'
  smtp_auth_password: 'password'

route:
  group_by: ['alertname', 'cluster']
  group_wait: 10s
  group_interval: 10s
  repeat_interval: 1h
  receiver: 'team-admins'
  
  routes:
    - match:
        severity: critical
      receiver: 'pagerduty'
      continue: true
    
    - match:
        severity: warning
      receiver: 'team-admins'

receivers:
  - name: 'team-admins'
    email_configs:
      - to: 'admin-team@company.com'
        headers:
          Subject: 'Readur Alert: {{ .GroupLabels.alertname }}'
  
  - name: 'pagerduty'
    pagerduty_configs:
      - service_key: 'your-pagerduty-key'
```

### Alert Rules

```yaml
# prometheus/alerts/readur.yml
groups:
  - name: readur
    rules:
      - alert: HighResponseTime
        expr: histogram_quantile(0.95, rate(readur_request_duration_seconds_bucket[5m])) > 1
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High response time on {{ $labels.instance }}"
          description: "95th percentile response time is {{ $value }}s"
      
      - alert: DatabaseDown
        expr: up{job="postgres"} == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "Database is down"
          description: "PostgreSQL database is not responding"
      
      - alert: HighOCRQueue
        expr: readur_ocr_queue_length > 1000
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "OCR queue backlog"
          description: "OCR queue has {{ $value }} pending items"
      
      - alert: DiskSpaceLow
        expr: node_filesystem_avail_bytes{mountpoint="/"} / node_filesystem_size_bytes{mountpoint="/"} < 0.1
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "Low disk space"
          description: "Only {{ $value | humanizePercentage }} disk space remaining"
```

## Performance Monitoring

### APM Integration

```python
# apm_config.py
from elasticapm import Client

# Configure APM
apm_client = Client({
    'SERVICE_NAME': 'readur',
    'SERVER_URL': 'http://apm-server:8200',
    'ENVIRONMENT': 'production',
    'SECRET_TOKEN': 'your-secret-token',
})

# Instrument Flask app
from elasticapm.contrib.flask import ElasticAPM
apm = ElasticAPM(app, client=apm_client)
```

### Custom Performance Metrics

```python
# performance_metrics.py
import time
from contextlib import contextmanager

@contextmanager
def track_performance(operation_name):
    start_time = time.time()
    try:
        yield
    finally:
        duration = time.time() - start_time
        metrics.record_operation_time(operation_name, duration)
        
        if duration > 1.0:  # Log slow operations
            logger.warning(f"Slow operation: {operation_name} took {duration:.2f}s")

# Usage
with track_performance("document_processing"):
    process_document(doc_id)
```

## Uptime Monitoring

### External Monitoring

```yaml
# uptime-kuma/docker-compose.yml
version: '3.8'

services:
  uptime-kuma:
    image: louislam/uptime-kuma:latest
    container_name: uptime-kuma
    volumes:
      - uptime-kuma_data:/app/data
    ports:
      - "3001:3001"
    restart: unless-stopped

volumes:
  uptime-kuma_data:
```

### Status Page Configuration

```nginx
# Public status page
server {
    listen 443 ssl;
    server_name status.readur.company.com;
    
    location / {
        proxy_pass http://localhost:3001;
        proxy_set_header Host $host;
    }
}
```

## Dashboard Examples

### Key Metrics Dashboard

```sql
-- Query for document processing stats
SELECT 
    DATE(created_at) as date,
    COUNT(*) as documents_processed,
    AVG(processing_time) as avg_processing_time,
    MAX(processing_time) as max_processing_time
FROM documents
WHERE created_at > NOW() - INTERVAL '30 days'
GROUP BY DATE(created_at)
ORDER BY date DESC;
```

### Real-time Monitoring

```javascript
// WebSocket monitoring dashboard
const ws = new WebSocket('wss://readur.company.com/ws/metrics');

ws.onmessage = (event) => {
    const metrics = JSON.parse(event.data);
    updateDashboard({
        activeUsers: metrics.active_users,
        queueLength: metrics.queue_length,
        responseTime: metrics.response_time,
        errorRate: metrics.error_rate
    });
};
```

## Troubleshooting Monitoring Issues

### Prometheus Not Scraping

```bash
# Check Prometheus targets
curl http://localhost:9090/api/v1/targets

# Verify metrics endpoint
curl http://localhost:8000/metrics

# Check network connectivity
docker network inspect monitoring
```

### Missing Metrics

```bash
# Debug metric collection
docker-compose exec readur python -c "
from prometheus_client import REGISTRY
for collector in REGISTRY._collector_to_names:
    print(collector)
"
```

### High Memory Usage

```bash
# Check Prometheus storage
du -sh /var/lib/prometheus

# Reduce retention
docker-compose exec prometheus promtool tsdb analyze /prometheus

# Clean old data
docker-compose exec prometheus promtool tsdb clean /prometheus
```

## Best Practices

### Monitoring Strategy

1. **Start Simple**: Begin with basic health checks and expand
2. **Alert Fatigue**: Only alert on actionable issues
3. **SLI/SLO Definition**: Define and track service level indicators
4. **Dashboard Organization**: Create role-specific dashboards
5. **Log Retention**: Balance storage costs with debugging needs
6. **Security**: Protect monitoring endpoints and dashboards
7. **Documentation**: Document alert runbooks and response procedures

### Maintenance

```bash
# Weekly maintenance tasks
#!/bin/bash

# Rotate logs
docker-compose exec readur logrotate -f /etc/logrotate.conf

# Clean up old metrics
curl -X POST http://localhost:9090/api/v1/admin/tsdb/clean_tombstones

# Backup Grafana dashboards
docker-compose exec grafana grafana-cli admin export-dashboard

# Update monitoring stack
docker-compose -f docker-compose.monitoring.yml pull
docker-compose -f docker-compose.monitoring.yml up -d
```

## Related Documentation

- [Performance Tuning](./performance.md)
- [Health Monitoring Guide](../health-monitoring-guide.md)
- [Backup Strategies](./backup.md)
- [Troubleshooting Guide](../troubleshooting.md)