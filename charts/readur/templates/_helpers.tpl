{{/* Name of the Secret holding ADMIN_PASSWORD and, optionally, JWT_SECRET. */}}
{{- define "readur.authSecretName" -}}
{{- if .Values.auth.existingSecret -}}
{{- .Values.auth.existingSecret -}}
{{- else -}}
{{- printf "%s-auth" .Release.Name | trunc 63 | trimSuffix "-" -}}
{{- end -}}
{{- end -}}

{{/*
Value of an environment variable set on the readur container, normalised from
either env form the common library accepts (a map, or a list of name/value
entries). A plain value is returned as a string; a reference (valueFrom) as
"<valueFrom>"; an unset variable as "". envFrom sources are not inspected.
Usage: include "readur.envValue" (dict "Values" .Values "name" "S3_ENABLED")
*/}}
{{- define "readur.envValue" -}}
{{- $env := dig "controllers" "main" "containers" "readur" "env" dict (.Values | toYaml | fromYaml) -}}
{{- $entry := "" -}}
{{- $found := false -}}
{{- if kindIs "map" $env -}}
  {{- if hasKey $env .name -}}
    {{- $entry = index $env .name -}}
    {{- $found = true -}}
  {{- end -}}
{{- else if kindIs "slice" $env -}}
  {{- range $env -}}
    {{- if and (kindIs "map" .) (eq (toString .name) $.name) -}}
      {{- $entry = . -}}
      {{- $found = true -}}
    {{- end -}}
  {{- end -}}
{{- end -}}
{{- if not $found -}}
{{- else if kindIs "map" $entry -}}
  {{- if hasKey $entry "value" -}}
    {{- toString $entry.value -}}
  {{- else if hasKey $entry "valueFrom" -}}
    {{- "<valueFrom>" -}}
  {{- end -}}
{{- else if not (kindIs "invalid" $entry) -}}
  {{- toString $entry -}}
{{- end -}}
{{- end -}}

{{/*
"true" when the readur container is configured to store documents in S3
(S3_ENABLED=true or STORAGE_BACKEND=s3, as src/config.rs reads them). A value
taken from a Secret (valueFrom) counts as configured, since it cannot be read
at render time.
*/}}
{{- define "readur.s3Enabled" -}}
{{- $enabled := include "readur.envValue" (dict "Values" .Values "name" "S3_ENABLED") | trim -}}
{{- $backend := include "readur.envValue" (dict "Values" .Values "name" "STORAGE_BACKEND") | trim -}}
{{- if or (eq (lower $enabled) "true") (eq $enabled "<valueFrom>") (eq (lower $backend) "s3") (eq $backend "<valueFrom>") -}}
true
{{- end -}}
{{- end -}}

{{/*
"true" when a database connection is configured on the readur container:
DATABASE_URL or POSTGRES_HOST in env, or any envFrom source (which may hold
them). Used for the install notes only.
*/}}
{{- define "readur.databaseConfigured" -}}
{{- $url := include "readur.envValue" (dict "Values" .Values "name" "DATABASE_URL") -}}
{{- $host := include "readur.envValue" (dict "Values" .Values "name" "POSTGRES_HOST") -}}
{{- $envFrom := dig "controllers" "main" "containers" "readur" "envFrom" list (.Values | toYaml | fromYaml) -}}
{{- if or $url $host $envFrom -}}
true
{{- end -}}
{{- end -}}

{{/*
Resource name the common library gives the Deployment and Service, for use
outside the library's own context (where .Values.global may be unset).
*/}}
{{- define "readur.fullname" -}}
{{- $global := .Values.global -}}
{{- if not (kindIs "map" $global) -}}{{- $global = dict -}}{{- end -}}
{{- $values := dict "global" $global "nameOverride" .Values.nameOverride "fullnameOverride" .Values.fullnameOverride -}}
{{- include "bjw-s.common.lib.chart.names.fullname" (dict "Values" $values "Release" .Release "Chart" .Chart) -}}
{{- end -}}
