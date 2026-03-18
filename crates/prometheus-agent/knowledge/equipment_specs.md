# Equipment Type Specifications

Reference specifications and sensor ranges for equipment types supported by
the Prometheus predictive-maintenance platform.

---

## Air Handling Unit (AHU)

### Description
Air handling units condition and circulate air through ductwork to occupied
spaces.  They typically include supply and return fans, heating and cooling
coils, filters, dampers, and an economiser section.

### Typical Sensor Inventory

| Sensor | Unit | Normal Range | Alarm Low | Alarm High | Sample Rate |
|--------|------|-------------|-----------|------------|-------------|
| supply_air_temp | degC | 12 - 24 | 5 | 35 | 1 min |
| return_air_temp | degC | 20 - 26 | 10 | 35 | 1 min |
| mixed_air_temp | degC | 10 - 28 | 0 | 40 | 1 min |
| outdoor_air_temp | degC | -20 - 45 | -30 | 55 | 5 min |
| supply_air_humidity | %RH | 30 - 65 | 15 | 85 | 1 min |
| fan_speed | RPM | 200 - 1800 | 0 | 2000 | 1 min |
| fan_power | kW | 0.5 - 75 | 0 | 90 | 1 min |
| damper_position | % | 0 - 100 | — | — | 1 min |
| filter_dp | Pa | 50 - 500 | 20 | 750 | 5 min |
| cooling_valve | % | 0 - 100 | — | — | 1 min |
| heating_valve | % | 0 - 100 | — | — | 1 min |
| supply_air_flow | L/s | 500 - 25000 | 0 | 30000 | 1 min |

### Key Relationships
- `supply_air_temp` is controlled by modulating `cooling_valve` and `heating_valve`.
- `mixed_air_temp` is a blend of `outdoor_air_temp` and `return_air_temp` governed by `damper_position`.
- `filter_dp` increases monotonically as the filter loads; reset on replacement.
- `fan_power` correlates with `fan_speed` cubed (affinity law).

---

## Boiler

### Description
Hot water or steam boilers combust fuel to heat water.  Monitored parameters
include water and flue-gas temperatures, pressures, combustion chemistry, and
fuel/water flow rates.

### Typical Sensor Inventory

| Sensor | Unit | Normal Range | Alarm Low | Alarm High | Sample Rate |
|--------|------|-------------|-----------|------------|-------------|
| supply_water_temp | degC | 70 - 95 | 50 | 110 | 1 min |
| return_water_temp | degC | 50 - 80 | 30 | 95 | 1 min |
| flue_gas_temp | degC | 120 - 250 | 80 | 350 | 1 min |
| steam_pressure | kPa | 100 - 1000 | 50 | 1100 | 30 sec |
| flame_signal | % | 50 - 100 | 20 | — | 1 sec |
| combustion_air_flow | m3/h | 100 - 5000 | 0 | 6000 | 1 min |
| fuel_flow_rate | m3/h | 5 - 500 | 0 | 600 | 1 min |
| stack_o2 | % | 2 - 6 | 1 | 10 | 1 min |
| water_level | mm | 100 - 300 | 50 | 350 | 30 sec |
| makeup_water_flow | L/min | 0 - 50 | — | 80 | 5 min |
| shell_temp_zone_1 | degC | 40 - 80 | — | 120 | 5 min |
| shell_temp_zone_2 | degC | 40 - 80 | — | 120 | 5 min |
| firing_rate | % | 0 - 100 | — | — | 1 min |

### Key Relationships
- `flue_gas_temp` rises with `firing_rate` and decreases when tubes are clean.
- `stack_o2` between 2-5 % indicates good combustion; outside this range suggests air imbalance.
- `supply_water_temp` - `return_water_temp` = delta-T; low delta-T indicates low load or flow issue.
- `flame_signal` must exceed 20 % for burner lockout avoidance.

---

## Chiller

### Description
Chillers remove heat from chilled water via a vapour-compression or
absorption cycle.  They are among the most complex HVAC equipment with
numerous interdependent sensors.

### Typical Sensor Inventory

| Sensor | Unit | Normal Range | Alarm Low | Alarm High | Sample Rate |
|--------|------|-------------|-----------|------------|-------------|
| evaporator_leaving_temp | degC | 5 - 10 | 2 | 15 | 30 sec |
| evaporator_entering_temp | degC | 10 - 16 | 5 | 22 | 30 sec |
| condenser_leaving_temp | degC | 28 - 38 | 20 | 45 | 30 sec |
| condenser_entering_temp | degC | 24 - 32 | 15 | 40 | 30 sec |
| compressor_amps | A | 50 - 800 | 0 | 1000 | 10 sec |
| compressor_discharge_temp | degC | 50 - 90 | 30 | 110 | 30 sec |
| evaporator_pressure | kPa | 200 - 600 | 100 | 700 | 30 sec |
| condenser_pressure | kPa | 600 - 1500 | 400 | 1800 | 30 sec |
| oil_pressure | kPa | 150 - 500 | 100 | 600 | 30 sec |
| refrigerant_level | % | 70 - 100 | 50 | — | 5 min |
| chilled_water_flow | L/s | 10 - 500 | 5 | 600 | 1 min |
| condenser_water_flow | L/s | 15 - 600 | 5 | 700 | 1 min |
| power_consumption | kW | 50 - 2000 | 0 | 2500 | 1 min |
| cop | — | 3.0 - 7.0 | 2.0 | 8.0 | 5 min |

### Key Relationships
- COP = cooling capacity / power consumption; declining COP indicates degradation.
- Condenser approach = `condenser_leaving_temp` - `condenser_entering_temp`; rising approach means fouling.
- Evaporator approach = `evaporator_entering_temp` - `evaporator_leaving_temp`; changes indicate fouling or flow issues.
- `compressor_amps` correlates with load; current above FLA indicates a problem.
- `evaporator_pressure` and `condenser_pressure` difference drives compressor work.

---

## Pump

### Description
Pumps circulate chilled water, hot water, or condenser water.  Key monitoring
focuses on vibration analysis, flow performance, and motor health.

### Typical Sensor Inventory

| Sensor | Unit | Normal Range | Alarm Low | Alarm High | Sample Rate |
|--------|------|-------------|-----------|------------|-------------|
| vibration_x | mm/s | 0 - 4.5 | — | 7.1 | 10 sec |
| vibration_y | mm/s | 0 - 4.5 | — | 7.1 | 10 sec |
| vibration_z | mm/s | 0 - 4.5 | — | 7.1 | 10 sec |
| flow_rate | L/s | 5 - 500 | 0 | 600 | 1 min |
| differential_pressure | kPa | 50 - 500 | 20 | 600 | 1 min |
| motor_current | A | 10 - 200 | 0 | 250 | 10 sec |
| bearing_temp | degC | 30 - 70 | 10 | 90 | 1 min |
| suction_pressure | kPa | 50 - 300 | 10 | 400 | 1 min |
| motor_temp | degC | 30 - 80 | 10 | 110 | 1 min |

### Key Relationships
- Vibration ISO 10816 limits: Good < 2.8 mm/s, Acceptable < 4.5, Alarm < 7.1, Danger >= 7.1.
- `flow_rate` vs `differential_pressure` should follow the pump curve; deviation indicates wear.
- `motor_current` at constant speed indicates load; rising current at same flow means degradation.
- `bearing_temp` above 80 degC warrants investigation; above 90 degC requires shutdown.

---

## Fan Coil Unit

### Description
Terminal units that provide local heating/cooling to individual zones.
Simple devices with a fan, coil, and control valve.

### Typical Sensor Inventory

| Sensor | Unit | Normal Range | Alarm Low | Alarm High | Sample Rate |
|--------|------|-------------|-----------|------------|-------------|
| discharge_air_temp | degC | 14 - 35 | 5 | 45 | 1 min |
| room_temp | degC | 20 - 26 | 15 | 32 | 1 min |
| room_humidity | %RH | 30 - 60 | 20 | 75 | 5 min |
| fan_speed | RPM | 0 - 1200 | — | 1400 | 1 min |
| valve_position | % | 0 - 100 | — | — | 1 min |
| return_air_temp | degC | 20 - 28 | 12 | 35 | 1 min |
| condensate_level | mm | 0 - 30 | — | 50 | 5 min |

### Key Relationships
- `discharge_air_temp` is governed by `valve_position` and water supply temperature.
- `room_temp` should converge toward setpoint; persistent offset indicates capacity or sensor issue.
- `condensate_level` should stay low in cooling mode; rising level indicates drain blockage.

---

## Steam System

### Description
Steam distribution systems deliver thermal energy from boilers to process
loads or HVAC coils.  Monitoring covers headers, distribution piping, traps,
pressure-reducing valves, and condensate return.

### Typical Sensor Inventory

| Sensor | Unit | Normal Range | Alarm Low | Alarm High | Sample Rate |
|--------|------|-------------|-----------|------------|-------------|
| header_pressure | kPa | 200 - 1000 | 100 | 1100 | 30 sec |
| header_temp | degC | 120 - 185 | 100 | 200 | 1 min |
| steam_flow | kg/h | 100 - 10000 | 0 | 12000 | 1 min |
| condensate_temp | degC | 80 - 100 | 50 | 110 | 1 min |
| condensate_flow | L/min | 5 - 200 | 0 | 250 | 1 min |
| feedwater_temp | degC | 80 - 105 | 60 | 110 | 1 min |
| deaerator_pressure | kPa | 3 - 20 | 1 | 30 | 1 min |
| blowdown_rate | L/min | 0 - 10 | — | 15 | 5 min |
| makeup_water_flow | L/min | 0 - 50 | — | 80 | 5 min |
| stack_temp | degC | 120 - 250 | 80 | 350 | 1 min |
| pipe_surface_temp | degC | 30 - 60 | — | 80 | 5 min |
| dissolved_o2 | ppb | 0 - 7 | — | 20 | 5 min |
| condensate_conductivity | uS/cm | 0 - 50 | — | 100 | 5 min |

### Key Relationships
- `header_pressure` must remain within design band; instability causes safety valve lifts.
- `condensate_temp` near saturation at header pressure indicates no sub-cooling (healthy trap).
- `dissolved_o2` above 7 ppb accelerates corrosion; indicates deaerator issue.
- `makeup_water_flow` should equal blowdown + losses; excess indicates leak or trap blow-through.
