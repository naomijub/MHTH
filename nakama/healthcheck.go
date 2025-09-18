package main

import (
	"context"
	"database/sql"
	"encoding/json"

	"github.com/heroiclabs/nakama-common/runtime"
)

type HealthcheckResponse struct {
	Success bool `json:"success"`
}

func HealthcheckRpc(ctx context.Context, logger runtime.Logger, db *sql.DB, nk runtime.NakamaModule, payload string) (string, error) {
	logger.Debug("Healthcheck Called - Payload: `%s`", payload)
	response := &HealthcheckResponse{Success: true}

	jsonResponse, err := json.Marshal(response)
	if err != nil {
		logger.Error("Error marshalling response: %v", err)
		return "", runtime.NewError("error marshalling response", 500)
	}
	return string(jsonResponse), nil
}
