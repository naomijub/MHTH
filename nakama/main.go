package main

import (
	"context"
	"database/sql"
	"time"

	"github.com/heroiclabs/nakama-common/runtime"
)

const (
	rpcHealthcheck = "healthcheck"
)

func InitModule(ctx context.Context, logger runtime.Logger, db *sql.DB, nk runtime.NakamaModule, initializer runtime.Initializer) error {
	logger.Debug("Hello World!")
	logger.Info("Server init modules MHTH")
	startTime := time.Now()

	if err := initializer.RegisterRpc(rpcHealthcheck, HealthcheckRpc); err != nil {
		logger.Error("Error registering rpc healthcheck: %v", err)
		return err
	}

	logger.Info("Module MHTH init complete: %dms", time.Since(startTime).Milliseconds())
	return nil
}
