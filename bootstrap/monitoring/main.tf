terraform {
  required_providers {
    grafana = {
      source  = "grafana/grafana"
      version = ">= 1.28.2"
    }
  }
}

variable "o11y_datasource_uid" {
  type = string
}

resource "grafana_folder" "folder" {
  title = "Ogmios"
}

resource "grafana_rule_group" "instance_is_down" {
  name             = "Ogmios is down"
  folder_uid       = grafana_folder.folder.uid
  interval_seconds = 60
  org_id           = 1

  rule {
    name           = "Ogmios is down"
    condition      = "B"
    for            = "5m"
    no_data_state  = "OK"
    exec_err_state = "OK"
    annotations = {
      description = "We are not receiving more metrics from a particular Ogmios instance.",
      summary     = "{{ range $k, $v := $values -}}\n{{ if (match \"A[0-9]+\" $k) -}}\nPod: {{ $v.Labels.pod }}\n{{ end }}\n{{ end }}"
    }

    data {
      ref_id         = "A"
      datasource_uid = var.o11y_datasource_uid

      relative_time_range {
        from = 3600
        to   = 0
      }

      model = jsonencode({
        editorMode    = "code",
        expr          = "count(avg_over_time(ogmios_connected{pod!~\"ogmios-vector-testnet-.*\"}[10m] offset 1h)) by (pod) unless count(avg_over_time(ogmios_connected{pod!~\"ogmios-vector-testnet-.*\"}[10m])) by (pod)",
        hide          = false,
        intervalMs    = 1000,
        legendFormat  = "__auto",
        maxDataPoints = 43200,
        range         = true,
        refId         = "A"
      })
    }

    data {
      ref_id         = "B"
      datasource_uid = "-100"

      relative_time_range {
        from = 3600
        to   = 0
      }

      model = jsonencode({
        conditions = [
          {
            evaluator = {
              params = [0]
              type   = "gt"
            },
            operator = {
              type = "and"
            },
            query = {
              params : [
                "A"
              ]
            },
            reducer = {
              params = [],
              type   = "count_non_null"
            },
            type = "query"
          }
        ],
        datasource = {
          type = "__expr__",
          uid  = "-100"
        },
        expression    = "A",
        hide          = false,
        intervalMs    = 1000,
        maxDataPoints = 43200,
        refId         = "B",
        type          = "classic_conditions"
      })
    }
  }
}
