job "commerce-v2" {
  datacenters = ["dc1"]
  type        = "service"

  group "commerce-v2-api" {
    count = 2

    network {
      mode = "bridge"

      port "grpc" {}
    }

    service {
      name = "commerce-v2-api"
      port = "grpc"

      connect {
        sidecar_service {
          proxy {
            upstreams {
              destination_name = "nats"
              local_bind_port = 4222
            }
            upstreams {
              destination_name = "postgres-sql"
              local_bind_port  = 5432
            }
          }
        }
      }

      check {
        type     = "grpc"
        interval = "20s"
        timeout  = "2s"
      }
    }

    task "commerce-v2-api" {
      driver = "docker"

      resources {
        cpu        = 100
        memory     = 256
        memory_max = 256
      }

      vault {
        policies = ["service-commerce_v2"]
      }

      template {
        destination = "${NOMAD_SECRETS_DIR}/.env"
        env         = true
        change_mode = "restart"
        data        = <<EOF
{{ with nomadVar "nomad/jobs/commerce-v2" }}
RUST_LOG='{{ .RUST_LOG }}'
{{ end }}

HOST='0.0.0.0:{{ env "NOMAD_PORT_grpc" }}'

NATS_HOST='{{ env "NOMAD_ADDR_nats" }}'
NATS_USER='{{- with nomadVar "nomad/jobs" -}}{{ .NATS_USER }}{{- end -}}'
NATS_PASSWORD='{{- with secret "kv2/data/services" -}}{{ .Data.data.NATS_PASSWORD }}{{- end -}}'

{{ with secret "database/static-creds/commerce_v2_user" }}
DATABASE_URL="postgresql://commerce_v2_user:{{ .Data.password }}@{{ env "NOMAD_UPSTREAM_IP_postgres-sql" }}:{{ env "NOMAD_UPSTREAM_PORT_postgres-sql" }}/commerce_v2"
{{ end }}

{{ with nomadVar "nomad/jobs/" }}
JWKS_HOST='{{ .JWKS_HOST }}'
JWKS_URL='{{ .JWKS_URL }}'
{{ end }}

{{ with nomadVar "nomad/jobs/commerce-v2" }}
S3_BUCKET_NAME='{{ .S3_BUCKET_NAME }}'
S3_BUCKET_ENDPOINT='{{ .S3_BUCKET_ENDPOINT }}'
S3_ACCESS_KEY_ID='{{ .S3_ACCESS_KEY_ID }}'
S3_MAX_ALLOWED_IMAGE_SIZE_BYTES='{{ .S3_MAX_ALLOWED_IMAGE_SIZE_BYTES }}'
S3_BASE_URL='{{ .S3_BASE_URL }}'
{{ end }}
{{ with secret "kv2/data/services/commerce_v2" }}
S3_SECRET_ACCESS_KEY='{{ .Data.data.S3_SECRET_ACCESS_KEY }}'
{{ end }}

{{ with secret "kv2/data/services/commerce_v2" }}
STRIPE_SECRET_KEY='{{ .Data.data.STRIPE_SECRET_KEY }}'
{{ end }}

{{ with nomadVar "nomad/jobs/commerce-v2" }}
DEFAULT_USER_QUOTA_MAX_ALLOWED_SIZE_BYTES='{{ .DEFAULT_USER_QUOTA_MAX_ALLOWED_SIZE_BYTES }}'
DEFAULT_PLATFORM_FEE_PERCENT='{{ .DEFAULT_PLATFORM_FEE_PERCENT }}'
DEFAULT_MINIMUM_PLATFORM_FEE_CENT='{{ .DEFAULT_MINIMUM_PLATFORM_FEE_CENT }}'
{{ end }}
EOF
      }

      config {
        image      = "__IMAGE__"
        force_pull = true
      }
    }
  }
}
