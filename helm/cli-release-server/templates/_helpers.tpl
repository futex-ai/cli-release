{{- define "cli-release-server.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{- define "cli-release-server.fullname" -}}
{{- if .Values.fullnameOverride -}}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" -}}
{{- else -}}
{{- $name := default .Chart.Name .Values.nameOverride -}}
{{- if contains $name .Release.Name -}}
{{- .Release.Name | trunc 63 | trimSuffix "-" -}}
{{- else -}}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" -}}
{{- end -}}
{{- end -}}
{{- end -}}

{{- define "cli-release-server.labels" -}}
helm.sh/chart: {{ printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | quote }}
app.kubernetes.io/name: {{ include "cli-release-server.name" . | quote }}
app.kubernetes.io/instance: {{ .Release.Name | quote }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
app.kubernetes.io/managed-by: {{ .Release.Service | quote }}
{{- end -}}

{{- define "cli-release-server.selectorLabels" -}}
app.kubernetes.io/name: {{ include "cli-release-server.name" . | quote }}
app.kubernetes.io/instance: {{ .Release.Name | quote }}
{{- end -}}

{{- define "cli-release-server.serverBind" -}}
0.0.0.0:8080
{{- end -}}

{{- define "cli-release-server.serviceAccountName" -}}
{{- if .Values.serviceAccount.create -}}
{{- default (include "cli-release-server.fullname" .) .Values.serviceAccount.name -}}
{{- else -}}
{{- default "default" .Values.serviceAccount.name -}}
{{- end -}}
{{- end -}}

{{- define "cli-release-server.secretName" -}}
{{- if .Values.secret.existingSecret -}}
{{- .Values.secret.existingSecret -}}
{{- else if .Values.secret.name -}}
{{- .Values.secret.name -}}
{{- else -}}
{{- printf "%s-env" (include "cli-release-server.fullname" .) -}}
{{- end -}}
{{- end -}}

{{- define "cli-release-server.validateGithubSecret" -}}
{{- if and .Values.config.releaseGithubRepository (not .Values.secret.existingSecret) (not .Values.secret.externalSecret.enabled) (not .Values.secret.values.releaseGithubToken) -}}
{{- fail "config.releaseGithubRepository requires secret.values.releaseGithubToken, secret.existingSecret, or secret.externalSecret.enabled" -}}
{{- end -}}
{{- end -}}

{{- define "cli-release-server.validateServerBind" -}}
{{- if and .Values.config.serverBind (ne .Values.config.serverBind (include "cli-release-server.serverBind" .)) -}}
{{- fail "config.serverBind is not configurable in the Helm chart; the chart-managed server bind is 0.0.0.0:8080" -}}
{{- end -}}
{{- end -}}
