local grafana = import 'grafonnet/grafana.libsonnet';
local dashboard = grafana.dashboard;
local row = grafana.row;
local singlestat = grafana.singlestat;
local prometheus = grafana.prometheus;
local template = grafana.template;

dashboard.new(
    'Scaphandre example dashboard',
    tags=['scaphandre', 'energy', 'power'],
    editable=true
)
.addTemplate(
    template.datasource(
        'PROMETHEUS_DS',
        'prometheus',
        'Prometheus',
        hide='label',
    )
)
.addTemplate(
    template.text(
        name='process_filter',
    )
)
.addRow(
  row.new(
      title='Per hosts',
  )
  .addPanel(
      grafana.graphPanel.new(
          title='Hosts power consumption',
          datasource='${PROMETHEUS_DS}',
          format='W',
          span=6,
          min=0
      )
      .addTarget(
          grafana.prometheus.target(
              'scaph_host_power_microwatts / 1000000',
              legendFormat='{{instance}}',
          )
      )
  )
  .addPanel(
      grafana.graphPanel.new(
          title='Hosts power consumption total (dynamic time range)',
          datasource='${PROMETHEUS_DS}',
          span=4,
          bars=true,
          format='Wh',
          x_axis_mode='series',
          min=0
      )
      .addTarget(
          grafana.prometheus.target(
              'sum(avg_over_time(scaph_host_power_microwatts[1h]))/1000000',
              legendFormat='total of hosts, during displayed time window',
              interval='1h'
          )
      )
   )
)
.addRow(
    row.new(
        title='Per CPU Sockets'
    )
    .addPanel(
        grafana.graphPanel.new(
            title='Socket power consumption',
            datasource='${PROMETHEUS_DS}',
            format='W',
            span=6,
            min=0
        )
        .addTarget(
            grafana.prometheus.target(
                'scaph_socket_power_microwatts / 1000000',
                legendFormat='{{instance}} Socket {{socket_id}}',
            )
        )
    )
) 
.addRow(
    row.new(
        title='Per process',
    )
    .addPanel(
        grafana.statPanel.new(
            title='Top process consumers',
            datasource='${PROMETHEUS_DS}',
        )
        .addTarget(
            grafana.prometheus.target(
                'sort_desc(topk(3, sum by (exe) (scaph_process_power_consumption_microwatts/1000000)))',
                legendFormat='{{exe}}',
            )
        )
    )
    .addPanel(
        grafana.graphPanel.new(
            title='Filtered process (process_filter) power, by exe',
            datasource='${PROMETHEUS_DS}',
            span=8,
            format='W',
            legend_rightSide=false,
            legend_alignAsTable=true,
            legend_sideWidth='30%',
            stack=true,
            min=0
        )
        .addTarget(
            grafana.prometheus.target(
                'scaph_process_power_consumption_microwatts{exe=~".*${process_filter}.*"}/1000000',
                legendFormat='{{ cmdline }}',
            )
        )
    )
)
