# HVAC Fault Catalog

Comprehensive catalog of common HVAC failure modes with detection signatures
for Prometheus anomaly detection models.

---

## Air Handling Units (AHU)

### AHU-001: Supply Fan Bearing Degradation
- **Severity:** High
- **Detection signature:** Increasing vibration amplitude on supply fan, gradual rise in motor current, elevated bearing temperature.
- **Lead time:** 2-6 weeks before failure
- **Key sensors:** `vibration_x`, `vibration_y`, `motor_current`, `bearing_temp`
- **Pattern:** Linear uptrend in vibration RMS with periodic spikes during high-load periods.

### AHU-002: Clogged Air Filter
- **Severity:** Medium
- **Detection signature:** Rising filter differential pressure beyond baseline, decreased supply air flow, increased fan energy consumption.
- **Lead time:** 1-4 weeks
- **Key sensors:** `filter_dp`, `supply_air_flow`, `fan_power`
- **Pattern:** Monotonic increase in `filter_dp` with inverse correlation to `supply_air_flow`.

### AHU-003: Frozen Cooling Coil
- **Severity:** Critical
- **Detection signature:** Supply air temperature drops below setpoint, cooling valve stuck open, chilled water delta-T collapse.
- **Lead time:** Hours to 1 day
- **Key sensors:** `supply_air_temp`, `cooling_valve`, `chw_supply_temp`, `chw_return_temp`
- **Pattern:** Sudden drop in supply air temp with near-zero chilled water delta-T.

### AHU-004: Economizer Damper Stuck Open
- **Severity:** Medium
- **Detection signature:** Mixed air temperature tracks outdoor air temperature regardless of damper command, excess heating/cooling energy.
- **Lead time:** Immediate (operational inefficiency)
- **Key sensors:** `mixed_air_temp`, `outdoor_air_temp`, `damper_position`, `damper_command`
- **Pattern:** `mixed_air_temp` closely tracks `outdoor_air_temp`; `damper_position` does not follow `damper_command`.

### AHU-005: Economizer Damper Stuck Closed
- **Severity:** Medium
- **Detection signature:** Mixed air temperature equals return air temperature, no free cooling even when outdoor conditions are favourable.
- **Lead time:** Immediate
- **Key sensors:** `mixed_air_temp`, `return_air_temp`, `outdoor_air_temp`, `damper_position`
- **Pattern:** `mixed_air_temp` equals `return_air_temp` when OAT is below RAT.

### AHU-006: Heating Coil Valve Leak
- **Severity:** Medium
- **Detection signature:** Supply air temperature higher than expected during cooling mode, hot water return temperature elevated.
- **Lead time:** Days to weeks
- **Key sensors:** `supply_air_temp`, `heating_valve`, `hw_return_temp`
- **Pattern:** Elevated SAT when heating valve commanded closed.

### AHU-007: VFD Failure / Degradation
- **Severity:** High
- **Detection signature:** Fan speed does not match VFD command, erratic speed fluctuations, overcurrent events.
- **Lead time:** Days to weeks
- **Key sensors:** `fan_speed`, `vfd_command`, `motor_current`, `vfd_fault_code`
- **Pattern:** Deviation between commanded and actual speed; current spikes.

### AHU-008: Humidity Sensor Drift
- **Severity:** Low
- **Detection signature:** Humidity readings diverge from expected values given temperature and dewpoint, simultaneous heating and humidifying.
- **Lead time:** Weeks to months
- **Key sensors:** `supply_air_humidity`, `return_air_humidity`, `dewpoint`
- **Pattern:** Gradual offset drift in humidity reading vs. calculated dewpoint.

### AHU-009: Supply Air Temperature Sensor Failure
- **Severity:** High
- **Detection signature:** SAT reading is constant (stuck) or shows physically impossible values, control loop oscillation.
- **Lead time:** Immediate
- **Key sensors:** `supply_air_temp`
- **Pattern:** Flat-line or out-of-range values (e.g. < 0 degC or > 60 degC).

### AHU-010: Belt Slippage on Fan
- **Severity:** Medium
- **Detection signature:** Reduced air flow despite normal fan speed command, intermittent vibration spikes, audible squealing (not detectable via sensors alone).
- **Lead time:** Days
- **Key sensors:** `supply_air_flow`, `fan_speed`, `vibration_x`
- **Pattern:** Air flow drops while fan speed remains constant; periodic vibration bursts.

---

## Chillers

### CHL-001: Refrigerant Leak
- **Severity:** Critical
- **Detection signature:** Declining suction pressure, rising discharge superheat, decreased cooling capacity, compressor short-cycling.
- **Lead time:** Days to weeks
- **Key sensors:** `evaporator_pressure`, `condenser_pressure`, `refrigerant_level`, `compressor_discharge_temp`
- **Pattern:** Gradual downtrend in evaporator pressure; uptrend in discharge temperature.

### CHL-002: Condenser Fouling
- **Severity:** Medium
- **Detection signature:** Rising condenser approach temperature, increased head pressure, higher compressor energy.
- **Lead time:** Weeks to months
- **Key sensors:** `condenser_leaving_temp`, `condenser_entering_temp`, `condenser_pressure`, `compressor_amps`
- **Pattern:** Increasing condenser approach (leaving temp minus entering temp).

### CHL-003: Evaporator Fouling
- **Severity:** Medium
- **Detection signature:** Rising evaporator approach temperature, decreased chilled water delta-T, reduced COP.
- **Lead time:** Weeks to months
- **Key sensors:** `evaporator_leaving_temp`, `evaporator_entering_temp`, `evaporator_pressure`, `cop`
- **Pattern:** Increasing evaporator approach; declining COP.

### CHL-004: Compressor Valve Leak
- **Severity:** High
- **Detection signature:** Reduced capacity at given speed, increased discharge temperature, elevated compressor current.
- **Lead time:** Weeks
- **Key sensors:** `compressor_discharge_temp`, `compressor_amps`, `cop`
- **Pattern:** Discharge temp rises while capacity drops.

### CHL-005: Oil Pressure Failure
- **Severity:** Critical
- **Detection signature:** Low oil differential pressure, compressor lockout.
- **Lead time:** Hours
- **Key sensors:** `oil_pressure`, `compressor_amps`
- **Pattern:** Sudden drop in oil pressure below safety threshold.

### CHL-006: Condenser Water Flow Loss
- **Severity:** Critical
- **Detection signature:** Condenser water delta-T spike, head pressure rise, high-pressure safety trip.
- **Lead time:** Minutes to hours
- **Key sensors:** `condenser_water_flow`, `condenser_leaving_temp`, `condenser_pressure`
- **Pattern:** Flow drops to zero or near-zero; rapid pressure rise.

### CHL-007: Chilled Water Flow Loss
- **Severity:** Critical
- **Detection signature:** Evaporator delta-T collapse, low-pressure safety trip, freeze protection alarm.
- **Lead time:** Minutes
- **Key sensors:** `chilled_water_flow`, `evaporator_leaving_temp`, `evaporator_pressure`
- **Pattern:** Flow drops; leaving temp plummets toward freeze point.

### CHL-008: Capacity Degradation
- **Severity:** Medium
- **Detection signature:** Unable to meet load at full speed, COP below historical baseline, extended run times.
- **Lead time:** Weeks to months
- **Key sensors:** `cop`, `compressor_amps`, `chilled_water_flow`, `evaporator_leaving_temp`
- **Pattern:** COP trending below rolling 90-day average.

### CHL-009: VFD Harmonic Distortion
- **Severity:** Low
- **Detection signature:** Motor current waveform distortion, intermittent VFD fault codes, elevated motor temperature.
- **Lead time:** Weeks
- **Key sensors:** `compressor_amps`, `motor_temp`, `vfd_fault_code`
- **Pattern:** THD (total harmonic distortion) increase in current signature.

### CHL-010: Surge Condition
- **Severity:** Critical
- **Detection signature:** Rapid pressure oscillation between evaporator and condenser, compressor noise, current instability.
- **Lead time:** Immediate
- **Key sensors:** `evaporator_pressure`, `condenser_pressure`, `compressor_amps`
- **Pattern:** High-frequency oscillation in pressure signals.

---

## Boilers

### BLR-001: Flame Failure
- **Severity:** Critical
- **Detection signature:** Flame signal loss, burner lockout, fuel valve closure.
- **Lead time:** Immediate
- **Key sensors:** `flame_signal`, `fuel_flow_rate`
- **Pattern:** Flame signal drops to zero during firing cycle.

### BLR-002: Low Water Condition
- **Severity:** Critical
- **Detection signature:** Water level below minimum, make-up water valve fully open, pressure drop.
- **Lead time:** Minutes to hours
- **Key sensors:** `water_level`, `makeup_water_flow`, `steam_pressure`
- **Pattern:** Water level trending below low-water cutoff setpoint.

### BLR-003: Tube Fouling / Scale Buildup
- **Severity:** Medium
- **Detection signature:** Rising flue gas temperature, decreased heat transfer efficiency, increased fuel consumption per unit steam.
- **Lead time:** Weeks to months
- **Key sensors:** `flue_gas_temp`, `stack_temp`, `fuel_flow_rate`, `steam_flow`
- **Pattern:** Flue gas temp increases at constant load; fuel-to-steam ratio worsens.

### BLR-004: Combustion Air Imbalance
- **Severity:** Medium
- **Detection signature:** High or low stack O2, CO spikes, poor combustion efficiency.
- **Lead time:** Days
- **Key sensors:** `stack_o2`, `combustion_air_flow`, `flue_gas_temp`
- **Pattern:** O2 outside 2-5 % optimal range; CO above 100 ppm.

### BLR-005: Refractory Deterioration
- **Severity:** High
- **Detection signature:** Elevated shell temperature, hot spots detected by surface thermocouples.
- **Lead time:** Weeks to months
- **Key sensors:** `shell_temp_zone_1`, `shell_temp_zone_2`, `shell_temp_zone_3`
- **Pattern:** Localised shell temperature rise above baseline.

### BLR-006: Safety Valve Weeping
- **Severity:** High
- **Detection signature:** Pressure drop below setpoint without load change, audible discharge (not sensor-detectable), makeup water increase.
- **Lead time:** Days
- **Key sensors:** `steam_pressure`, `makeup_water_flow`
- **Pattern:** Pressure sag with unexplained makeup water demand.

### BLR-007: Feedwater Pump Degradation
- **Severity:** High
- **Detection signature:** Decreased feedwater flow at given speed, increased pump motor current, water level oscillation.
- **Lead time:** Weeks
- **Key sensors:** `feedwater_flow`, `feedwater_pump_current`, `water_level`
- **Pattern:** Flow declining at constant speed; current rising.

### BLR-008: Burner Modulation Failure
- **Severity:** Medium
- **Detection signature:** Output stuck at one firing rate, cycling between full fire and off, pressure oscillation.
- **Lead time:** Days
- **Key sensors:** `firing_rate`, `steam_pressure`, `fuel_flow_rate`
- **Pattern:** Step-function firing rate instead of smooth modulation.

---

## Pumps

### PMP-001: Impeller Erosion
- **Severity:** High
- **Detection signature:** Decreased flow at rated speed, increased vibration, reduced differential pressure.
- **Lead time:** Weeks to months
- **Key sensors:** `flow_rate`, `differential_pressure`, `vibration_x`, `vibration_y`
- **Pattern:** Gradual decline in flow and head at constant speed.

### PMP-002: Bearing Wear
- **Severity:** High
- **Detection signature:** Elevated vibration (especially axial), rising bearing temperature, audible noise increase.
- **Lead time:** 2-8 weeks
- **Key sensors:** `vibration_x`, `vibration_y`, `vibration_z`, `bearing_temp`
- **Pattern:** Upward trend in vibration RMS with characteristic bearing defect frequencies.

### PMP-003: Seal Leak
- **Severity:** Medium
- **Detection signature:** Visible leakage (manual inspection), decreasing suction pressure if severe, moisture detection.
- **Lead time:** Days to weeks
- **Key sensors:** `suction_pressure`, `seal_leak_detector`
- **Pattern:** Gradual suction pressure decline or moisture alarm.

### PMP-004: Cavitation
- **Severity:** High
- **Detection signature:** High-frequency vibration, noise, fluctuating flow, pitting damage over time.
- **Lead time:** Immediate to days
- **Key sensors:** `vibration_x`, `suction_pressure`, `flow_rate`
- **Pattern:** High-frequency vibration spikes when suction pressure drops below NPSH requirement.

### PMP-005: Motor Insulation Breakdown
- **Severity:** Critical
- **Detection signature:** Increasing motor current, elevated winding temperature, earth-leakage current rise.
- **Lead time:** Days to weeks
- **Key sensors:** `motor_current`, `motor_temp`, `insulation_resistance`
- **Pattern:** Current and temperature rising at constant load.

### PMP-006: Coupling Misalignment
- **Severity:** Medium
- **Detection signature:** Elevated radial and axial vibration at 1x and 2x running speed, coupling temperature rise.
- **Lead time:** Weeks
- **Key sensors:** `vibration_x`, `vibration_y`, `coupling_temp`
- **Pattern:** Dominant 2x vibration component.

---

## Fan Coil Units

### FCU-001: Clogged Coil
- **Severity:** Medium
- **Detection signature:** Reduced heating/cooling output, increased delta-T across coil, fan running at higher speed to compensate.
- **Lead time:** Weeks
- **Key sensors:** `discharge_air_temp`, `room_temp`, `fan_speed`
- **Pattern:** Room temp drifts from setpoint despite fan at full speed.

### FCU-002: Stuck Valve
- **Severity:** Medium
- **Detection signature:** Output does not change with valve command, room temperature cannot reach setpoint.
- **Lead time:** Immediate
- **Key sensors:** `valve_position`, `valve_command`, `discharge_air_temp`
- **Pattern:** Valve position constant regardless of command signal.

### FCU-003: Fan Motor Failure
- **Severity:** High
- **Detection signature:** Zero air flow, motor overcurrent trip, room temperature drifts.
- **Lead time:** Hours to immediate
- **Key sensors:** `fan_speed`, `motor_current`, `room_temp`
- **Pattern:** Fan speed zero; room temp diverges from setpoint.

### FCU-004: Thermostat / Sensor Failure
- **Severity:** Medium
- **Detection signature:** Room temperature reading stuck or erratic, control actions inconsistent with occupant comfort.
- **Lead time:** Immediate
- **Key sensors:** `room_temp`, `room_humidity`
- **Pattern:** Flat-line or noisy room temp that does not correlate with valve / fan state.

### FCU-005: Condensate Drain Blockage
- **Severity:** Medium
- **Detection signature:** Condensate overflow alarm, elevated humidity near unit, water damage risk.
- **Lead time:** Days
- **Key sensors:** `condensate_level`, `room_humidity`
- **Pattern:** Condensate level rising above normal with simultaneous humidity spike.

---

## Steam Systems

### STM-001: Steam Trap Failure (Stuck Open)
- **Severity:** Medium
- **Detection signature:** Live steam passing to condensate return, elevated condensate temperature, energy waste.
- **Lead time:** Immediate (energy impact)
- **Key sensors:** `condensate_temp`, `trap_discharge_temp`, `steam_flow`
- **Pattern:** Trap discharge temp equals or exceeds upstream steam temp.

### STM-002: Steam Trap Failure (Stuck Closed)
- **Severity:** High
- **Detection signature:** Condensate backup, water hammer risk, reduced heat exchange.
- **Lead time:** Hours
- **Key sensors:** `condensate_temp`, `trap_discharge_temp`, `condensate_flow`
- **Pattern:** Near-zero flow through trap; upstream condensate temp drops.

### STM-003: Pressure Reducing Valve Malfunction
- **Severity:** High
- **Detection signature:** Downstream pressure out of specification (too high or too low), valve hunting.
- **Lead time:** Hours
- **Key sensors:** `header_pressure`, `downstream_pressure`, `prv_position`
- **Pattern:** Downstream pressure oscillates or sits outside setpoint band.

### STM-004: Insulation Degradation
- **Severity:** Low
- **Detection signature:** Elevated pipe surface temperature, increased heat loss, higher fuel consumption.
- **Lead time:** Months
- **Key sensors:** `pipe_surface_temp`, `ambient_temp`, `fuel_flow_rate`
- **Pattern:** Surface temp rises above insulated baseline at constant load.

### STM-005: Deaerator Malfunction
- **Severity:** High
- **Detection signature:** Elevated dissolved O2 in feedwater, increased corrosion potential, deaerator pressure/temperature anomaly.
- **Lead time:** Days
- **Key sensors:** `deaerator_pressure`, `feedwater_temp`, `dissolved_o2`
- **Pattern:** O2 levels above 7 ppb; deaerator temp below saturation.

### STM-006: Blowdown Valve Failure
- **Severity:** Medium
- **Detection signature:** Continuous blowdown exceeding setpoint, elevated makeup water demand, energy waste.
- **Lead time:** Days
- **Key sensors:** `blowdown_rate`, `makeup_water_flow`, `conductivity`
- **Pattern:** Blowdown rate stuck above target; conductivity does not decrease.

### STM-007: Header Pressure Instability
- **Severity:** High
- **Detection signature:** Pressure swings beyond normal band, boiler cycling, safety valve lifts.
- **Lead time:** Hours
- **Key sensors:** `header_pressure`, `steam_flow`, `firing_rate`
- **Pattern:** Pressure oscillation amplitude exceeds +/- 10 % of setpoint.

### STM-008: Condensate Return Contamination
- **Severity:** Medium
- **Detection signature:** Elevated conductivity or pH anomaly in condensate return, chemical treatment increase.
- **Lead time:** Days
- **Key sensors:** `condensate_conductivity`, `condensate_ph`, `condensate_temp`
- **Pattern:** Conductivity spikes above baseline.

---

## Cross-Equipment Faults

### GEN-001: Power Quality Issue
- **Severity:** Medium
- **Detection signature:** Voltage sag/swell, frequency deviation, VFD/drive faults across multiple equipment.
- **Lead time:** Immediate
- **Key sensors:** `supply_voltage`, `supply_frequency`, `vfd_fault_code`
- **Pattern:** Correlated VFD faults across multiple systems.

### GEN-002: Communication / BACnet Failure
- **Severity:** Low (but operationally impactful)
- **Detection signature:** Stale sensor values (no change over extended period), communication timeout alarms.
- **Lead time:** Immediate
- **Key sensors:** All (staleness detection)
- **Pattern:** Multiple sensors reporting identical values across consecutive intervals.

### GEN-003: Ambient Condition Anomaly
- **Severity:** Low
- **Detection signature:** Outdoor air temperature sensor reading inconsistent with weather service data.
- **Lead time:** Immediate
- **Key sensors:** `outdoor_air_temp`, `outdoor_humidity`
- **Pattern:** OAT diverges from weather API by > 5 degC for > 30 minutes.

### GEN-004: Simultaneous Heating and Cooling
- **Severity:** Medium
- **Detection signature:** Both heating and cooling valves open on the same AHU or zone, wasted energy.
- **Lead time:** Immediate
- **Key sensors:** `heating_valve`, `cooling_valve`
- **Pattern:** Both valves > 20 % open simultaneously for > 15 minutes.

### GEN-005: Scheduled vs Actual Occupancy Mismatch
- **Severity:** Low
- **Detection signature:** HVAC operating in occupied mode during unoccupied hours or vice versa.
- **Lead time:** Immediate
- **Key sensors:** `occupancy_schedule`, `fan_status`, `room_temp`
- **Pattern:** Fan running during scheduled unoccupied period.
