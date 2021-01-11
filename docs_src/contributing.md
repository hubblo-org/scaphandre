# Contributing guide

If you are reading this, you may be  to contribute. Just for that, a big thank you ! 👏

Feel free to propose pull requests, or open new [discussions](https://github.com/hubblo-org/scaphandre/discussions) or [issues](https://github.com/hubblo-org/scaphandre/issues) at will. Scaphandre is a collaborative project and all opinions and propositions shall be heard and studied. The contributions will be received with kindness, gratitude and with an open mind. Remember that we are all [dwarfs standing on the shoulders of giants](https://en.wikipedia.org/wiki/Standing_on_the_shoulders_of_giants). We all have to learn from others and to give back, with due mutual respect.
### Code of conduct

This project adheres to the Rust Code of Conduct, which [can be found here](https://www.rust-lang.org/conduct.html).
### Ways to contribute

Contributions may take multiple forms:
- 💻 **code**, of course, but not only (there is a lot more !)
- 📖 **documentation**
- 🎤 Any help on **communication**: writing blog posts, speaking about scaphandre in conferences, speaking and writing about the responsibility of tech to be sustainable as well !
- 🧬 **structuring the project** and the **community** is also a very important topic. Feel free to propose help, or start [discussions](https://github.com/hubblo-org/scaphandre/discussions) about that.

This project intends to unite a lot of people to have a lot of positive impact. Any action going helping us to get there will be very much appreciated ! 🎉
### Contact

Discussions and questions about the project are welcome on [gitter](https://gitter.im/hubblo-org/scaphandre?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge&utm_content=badge) or by [email](mailto://bpetit@hubblo.org?Subject=About%20Scaphandre).
### Contribution guidelines

This project intends to use [conventionnal commit messages](https://conventionalcommits.org/) and the [gitflow](https://nvie.com/posts/a-successful-git-branching-model/) workflow.

Scaphandre is a not only a tool, but a framework. Modules dedicated to collect energy comsumption data from the host are called [**Sensors**](docs/sensors).
Modules that are dedicated to send this data to a given channel or remote system are called [**Exporters**](docs/exporters). New Sensors and Exporters are going to be created and all contributions are welcome. For more on the internal structure please jump [here](explanations/internal-structure.md).