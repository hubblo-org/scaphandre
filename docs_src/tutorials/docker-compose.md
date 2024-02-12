# Run a complete stack with docker-compose

If you'd like to try scaphandre and see the data in a decent visualization tool, there is a docker-compose in the repository to install scaphandre along side with [prometheus](https://prometheus.io) and [grafana](https://grafana.com).

Once you have cloned the repository, just move to the docker-compose folder and run the stack:

    cd docker-compose
    docker-compose up -d

Be warned: the sample stack runs scaphandre as a privileged container. Otherwise apparmor or equivalents might complain about ptrace calls on the host. See [#135](https://github.com/hubblo-org/scaphandre/issues/135).

Grafana will be available at `http://localhost:3000`, the default username is `admin` and the password is `secret`.

Refresh the dashboard after 30s or enable auto-refresh and you should see the data filling the graphs.

The `process_filter` textbox on the top of the graph allows you to look at the power consumption of a single application or service. Type the name of the program you want to look at and press enter. In the `Filtered process (process_filter) power, by exe` graph, on the 3rd line, you should now see the power cosumption of the given program.

To remove the stack just run this command in the same folder:

    docker-compose down
