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
          span=4,
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
  .addPanel(
      grafana.graphPanel.new(
          title='Host load average',
          datasource='${PROMETHEUS_DS}',
          span=4,
          format='',
          min=0
      )
      .addTarget(
          grafana.prometheus.target(
              'scaph_host_load_avg_one',
              legendFormat='load_avg_1',
          )
      )
      .addTarget(
          grafana.prometheus.target(
              'scaph_host_load_avg_five',
              legendFormat='load_avg_5',
          )
      )
      .addTarget(
          grafana.prometheus.target(
              'scaph_host_load_avg_fifteen',
              legendFormat='load_avg_15',
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
            span=4,
            min=0
        )
        .addTarget(
            grafana.prometheus.target(
                'scaph_socket_power_microwatts / 1000000',
                legendFormat='{{instance}} Socket {{socket_id}}',
            )
        )
    )
    .addPanel(
        grafana.graphPanel.new(
            title='scaph_self_cpu',
            datasource='${PROMETHEUS_DS}',
            format='%',
            span=4,
            min=0
        )
        .addTarget(
            grafana.prometheus.target(
                'scaph_self_cpu_usage_percent',
                legendFormat='{{__name__}}',
            )
        )
    )
    .addPanel(
        grafana.graphPanel.new(
            title='Socket power consumption',
            datasource='${PROMETHEUS_DS}',
            format='Bytes',
            span=4,
            min=0
        )
        .addTarget(
            grafana.prometheus.target(
                'scaph_self_memory_bytes',
                legendFormat='{{__name__}}',
            )
        )
        .addTarget(
            grafana.prometheus.target(
                'scaph_self_memory_virtual_bytes',
                legendFormat='{{__name__}}',
            )
        )
    )
) 
.addRow(
    row.new(
        title='Per process',
    )
    .addPanel(
        grafana.graphPanel.new(
            title='Filtered process (process_filter) power, by exe',
            datasource='${PROMETHEUS_DS}',
            span=3,
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
    .addPanel(
        grafana.graphPanel.new(
            title='scaph_process_cpu',
            datasource='${PROMETHEUS_DS}',
            span=3,
            format='%',
            legend_rightSide=false,
            legend_alignAsTable=true,
            legend_sideWidth='30%',
            stack=true,
            min=0
        )
        .addTarget(
            grafana.prometheus.target(
                'scaph_process_cpu_usage_percentage{exe=~".*${process_filter}.*"}',
                legendFormat='{{ cmdline }}',
            )
        )
    )
    .addPanel(
        grafana.graphPanel.new(
            title='scaph_process_mem',
            datasource='${PROMETHEUS_DS}',
            span=3,
            format='bytes',
            legend_rightSide=false,
            legend_alignAsTable=true,
            legend_sideWidth='30%',
            stack=true,
            min=0
        )
        .addTarget(
            grafana.prometheus.target(
                'scaph_process_memory_bytes{exe=~".*${process_filter}.*"}',
                legendFormat='{{ cmdline }}',
            )
        )
    )
    .addPanel(
        grafana.graphPanel.new(
            title='scaph_process_mem_virtual',
            datasource='${PROMETHEUS_DS}',
            span=3,
            format='bytes',
            legend_rightSide=false,
            legend_alignAsTable=true,
            legend_sideWidth='30%',
            stack=true,
            min=0
        )
        .addTarget(
            grafana.prometheus.target(
                'scaph_process_memory_virtual_bytes{exe=~".*${process_filter}.*"}',
                legendFormat='{{ cmdline }}',
            )
        )
    )
)
