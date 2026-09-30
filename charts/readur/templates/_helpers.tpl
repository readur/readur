{{/* Name of the Secret holding JWT_SECRET and ADMIN_PASSWORD. */}}
{{- define "readur.authSecretName" -}}
{{- if .Values.auth.existingSecret -}}
{{- .Values.auth.existingSecret -}}
{{- else -}}
{{- printf "%s-auth" .Release.Name | trunc 63 | trimSuffix "-" -}}
{{- end -}}
{{- end -}}
