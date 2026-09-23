idf_component_register(
    SRCS "hmi_board.c"
    INCLUDE_DIRS "include"
    REQUIRES codec_board esp_codec_dev esp_driver_i2c esp_driver_sdmmc esp_driver_usb_serial_jtag fatfs esp_adc lwip
)
