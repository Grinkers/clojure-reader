#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

#include "pico/stdlib.h"
#include "hardware/gpio.h"
#include "hardware/adc.h"

void some_edn(char* edn);

int main() {
    stdio_init_all();
    const uint LED_PIN = 25;
    gpio_init(LED_PIN);
    gpio_set_dir(LED_PIN, GPIO_OUT);

    adc_init();
    // Make sure GPIO is high-impedance, no pullups etc
    adc_gpio_init(26);
    // Select ADC input 0 (GPIO26)
    adc_select_input(4);
    adc_set_temp_sensor_enabled(true);

    char buf[200];
    while (1) {
        sleep_ms(500);
        gpio_put(LED_PIN, 1);

        // 12-bit conversion, assume max value == ADC_VREF == 3.3 V
        const float conversion_factor = 3.3f / (1 << 12);
        uint16_t result = adc_read();
        printf("Raw value: 0x%03x, voltage: %f V\n", result, result * conversion_factor);

        float temperature = 27 - (((result * conversion_factor) - 0.706) / 0.001721);
        printf("Internal Temperature: %.2f degrees Celsius\n", temperature);

        sprintf(buf, "{:temp %.2f :foo #{1 2 3 42}}", temperature);
        some_edn(buf);

        sleep_ms(500);
        gpio_put(LED_PIN, 0);
    }
}
