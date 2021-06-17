package main

import (
	"github.com/spf13/viper"
)

type Config struct {
	UsernameBase string `mapstructure:"USERNAME_BASE"`
	PostgresUrl  string `mapstructure:"POSTGRES_URL"`
	EnableScouts bool   `mapstructure:"ENABLE_SCOUTS"`
	EnableTrader bool   `mapstructure:"ENABLE_TRADER"`
	EnableReset  bool   `mapstructure:"ENABLE_RESET"`
}

func LoadConfig() (Config, error) {
	viper.AddConfigPath(".")
	viper.SetConfigFile(".env")
	viper.SetConfigType("env")

	viper.AutomaticEnv()

	err := viper.ReadInConfig()
	if err != nil {
		return Config{}, err
	}

	c := Config{}
	err = viper.Unmarshal(&c)
	if err != nil {
		return Config{}, err
	}

	return c, nil
}
