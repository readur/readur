# Readur Helm Chart

This Helm chart deploys Readur on Kubernetes using the [bjw-s common library chart](https://github.com/bjw-s/helm-charts/tree/main/charts/library/common).

## Installation

```bash
helm repo add readur https://readur.github.io/charts
helm install readur readur/readur
```

## Configuration

### JWT Secret

The JWT secret is automatically generated and persisted if not provided. You have three options:

1. **Auto-generation (Recommended)**: Don't set any JWT configuration, and a secure secret will be auto-generated
2. **Custom value**: Set `jwtSecret.value` in your values
3. **Existing secret**: Reference an existing Kubernetes secret with `jwtSecret.existingSecret`

```yaml
# Option 1: Auto-generate (default)
jwtSecret:
  existingSecret: ""
  value: ""

# Option 2: Provide custom value
jwtSecret:
  value: "your-secure-secret-here"

# Option 3: Use existing Kubernetes secret
jwtSecret:
  existingSecret: "my-jwt-secret"
```

The auto-generated secret is preserved across upgrades using the `helm.sh/resource-policy: keep` annotation.

### Database Configuration

Configure the database connection using either a direct URL or an existing secret:

```yaml
# Option 1: Direct URL (not recommended for production)
database:
  url: "postgresql://user:password@postgres/readur"

# Option 2: Use existing secret (recommended)
database:
  existingSecret: "readur-database-secret"
```

If using an existing secret, it should contain a `DATABASE_URL` key.

### Persistence

The chart configures two persistent volumes:

```yaml
persistence:
  uploads:
    enabled: true
    size: 10Gi
    storageClass: ""  # Uses default if not specified
  
  watch:
    enabled: true
    size: 5Gi
    storageClass: ""
```

### Ingress

Enable ingress to expose Readur:

```yaml
ingress:
  main:
    enabled: true
    className: nginx
    hosts:
      - host: readur.example.com
        paths:
          - path: /
            pathType: Prefix
    tls:
      - secretName: readur-tls
        hosts:
          - readur.example.com
```

## Security Considerations

1. **JWT Secret**: The auto-generated JWT secret is stored in a Kubernetes Secret and persists across upgrades
2. **Database Credentials**: Use Kubernetes Secrets for database credentials in production
3. **File Permissions**: An init container sets proper permissions for upload/watch directories
4. **Non-root User**: The container runs as UID 1000 (non-root) for security

## Upgrading

When upgrading the chart, the JWT secret is preserved automatically. If you need to rotate the secret:

1. Delete the existing secret: `kubectl delete secret <release-name>-jwt`
2. Upgrade the chart: `helm upgrade readur readur/readur`

## Full Configuration

See [values.yaml](values.yaml) for all available configuration options.