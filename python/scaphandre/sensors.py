from dataclasses import dataclass

from .scaphandre import RawScaphandre


@dataclass
class EnergyRecord:
    """
    Energy record measured by Scaphandre
    """

    timestamp: str
    value: str
    unit: str


@dataclass(init=False)
class Scaphandre:
    """
    Scaphandre, a metrology agent dedicated to electrical power consumption metrics.
    """

    sensor_name: str

    def __init__(
        self,
        is_virtual_machine: bool = False,
        buffer_per_socket_max_kbytes: int = 8,
        buffer_per_domain_max_kbytes: int = 8,
    ):
        """
        Init Scaphandre

        :param is_virtual_machine: running on a virtual machine for powercap_rapl sensor
        :param buffer_per_socket_max_kbytes: max buffer per socket in kbytes  for powercap_rapl sensor
        :param buffer_per_domain_max_kbytes: max buffer per domain in kbytes  for powercap_rapl sensor
        """
        self._scaphandre = RawScaphandre(
            buffer_per_socket_max_kbytes,
            buffer_per_domain_max_kbytes,
            is_virtual_machine,
        )
        self.name = self._scaphandre.sensor_name

    def is_compatible(self) -> bool:
        """
        Check if Scaphandre has a sensor available and valid depending on the hardware context.

        :return: a sensor is available and valid
        """
        return self._scaphandre.is_compatible()

    def get_energy_consumption_measures(self) -> EnergyRecord:
        """
        Get the energy records from Scaphandre.

        :return: the energy record measured
        """
        return self._scaphandre.get_energy_consumption_measures()
