cmake_minimum_required(VERSION 3.13)
set(CMAKE_C_STANDARD 11)

# initialize the SDK based on PICO_SDK_PATH
# note: this must happen before project()
include(pico_sdk_import.cmake)

project(hello_edn)

# initialize the Raspberry Pi Pico SDK
pico_sdk_init()

add_executable(hello_edn
  hello_edn.c
)

add_custom_target(
  rustlib
  COMMAND cd ${CMAKE_SOURCE_DIR}/pico-edn && cargo build --release --target thumbv6m-none-eabi
)
add_dependencies(hello_edn rustlib)

# Pull in our pico_stdlib which aggregates commonly used features
target_link_libraries(
  hello_edn
  pico_stdlib hardware_adc
  ${CMAKE_SOURCE_DIR}/pico-edn/target/thumbv6m-none-eabi/release/libpicoedn.a
  )

# enable usb output, disable uart output
pico_enable_stdio_usb(hello_edn 1)
pico_enable_stdio_uart(hello_edn 0)

# create map/bin/hex file etc.
pico_add_extra_outputs(hello_edn)
