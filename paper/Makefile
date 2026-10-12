TECTONIC ?= tectonic
PYTHON ?= python3
BUILD_DIR ?= build

.PHONY: all draft heterogeneity check-tba final clean

all: draft

heterogeneity:
	$(PYTHON) generate_heterogeneity.py

check-tba:
	$(PYTHON) check_tba.py

draft:
	$(PYTHON) generate_heterogeneity.py
	$(PYTHON) check_tba.py
	mkdir -p $(BUILD_DIR)
	$(TECTONIC) --outdir $(BUILD_DIR) main.tex

final:
	$(PYTHON) generate_heterogeneity.py
	$(PYTHON) check_tba.py --final
	mkdir -p $(BUILD_DIR)
	$(TECTONIC) --outdir $(BUILD_DIR) main.tex

clean:
	rm -rf $(BUILD_DIR)
