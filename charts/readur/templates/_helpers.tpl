{{/* Name of the Secret holding ADMIN_PASSWORD and, optionally, JWT_SECRET. */}}
{{- define "readur.authSecretName" -}}
{{- if .Values.auth.existingSecret -}}
{{- .Values.auth.existingSecret -}}
{{- else -}}
{{- printf "%s-auth" .Release.Name | trunc 63 | trimSuffix "-" -}}
{{- end -}}
{{- end -}}
