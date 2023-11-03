package promremotew

import (
	"bytes"
	"fmt"
	"io"
	"net/http"
	"time"

	"go.uber.org/zap"

	"github.com/golang/snappy"
)

func scrapeLocalMetrics() ([]byte, error) {
	// The idea is to grab all the metrics from localhost:6060/metrics,
	// and then send them to Grafana.
	const metricsPort = 6060
	metricsURL := fmt.Sprintf("http://localhost:%d/metrics", metricsPort)
	req, err := http.NewRequest(http.MethodGet, metricsURL, nil)
	if err != nil {
		// Could not create request
		return nil, err
	}
	res, err := http.DefaultClient.Do(req)
	if err != nil {
		// Error creating http request
		return nil, err
	}
	// TODO: Check status code 200 vs error codes?
	// fmt.Printf("client: status code: %d\n", res.StatusCode)
	resBody, err := io.ReadAll(res.Body)
	if err != nil {
		// Could not read response body
		return nil, err
	}
	// fmt.Printf("client: response body: %s\n", resBody)
	return resBody, nil
}

func scrapeAndSendLocalMetrics(url *string, user *string, key *string, logger *zap.Logger) {
	metrics, error := scrapeLocalMetrics()
	if error != nil {
		logger.Error("Could not scrape local metrics", zap.Error(error))
		return
	}
	input := bytes.NewReader(metrics)
	labels := map[string]string{"node_name": "testNode"}

	writeRequest, err := MetricTextToWriteRequest(input, labels)
	if err != nil {
		logger.Error("Could not create write request", zap.Error(err))
		return
	}
	raw, err := writeRequest.Marshal()
	if err != nil {
		logger.Error("Could not marshal write request", zap.Error(err))
		return
	}
	oSnap := snappy.Encode(nil, raw)
	bodyReader := bytes.NewReader(oSnap)

	// Create the http request
	requestURL := fmt.Sprintf("https://%s:%s@%s", *user, *key, *url)
	req, err := http.NewRequest(http.MethodPost, requestURL, bodyReader)
	if err != nil {
		logger.Error("Could not create request", zap.Error(err))
		return
	}
	req.Header.Set("Content-Encoding", "snappy")
	req.Header.Set("Content-Type", "application/x-protobuf")
	req.Header.Set("User-Agent", "Guardian/2.23.12")
	req.Header.Set("X-Prometheus-Remote-Write-Version", "0.1.0")

	res, err := http.DefaultClient.Do(req)
	if err != nil {
		logger.Error("Error creating http request", zap.Error(err))
		return
	}

	// TODO:  Do we care if grafana sends us back a 200?
	logger.Debug("Grafana result", zap.Int("status code", res.StatusCode))
	// resBody, err := io.ReadAll(res.Body)
	// if err != nil {
	// 	fmt.Printf("client: could not read response body: %s\n", err)
	// 	return
	// }
	// fmt.Printf("client: response body: %s\n", resBody)
}

func StartPrometheusScraper(promRemoteURL *string, promRemoteUser *string, promRemoteKey *string, logger *zap.Logger) {
	promLogger := logger.With(zap.String("component", "prometheus_scraper"))
	for {
		time.Sleep(15 * time.Second)
		scrapeAndSendLocalMetrics(promRemoteURL, promRemoteUser, promRemoteKey, promLogger)
	}
}
