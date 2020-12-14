QUICKJS := quickjs
LODASH := lodash

LIBS := $(QUICKJS)/libquickjs.a
OBJS := jsrunner.o stdlib.js.o lodash.js.o

QJSC := $(QUICKJS)/qjsc.wasm
run_qjsc = wasmer run --dir . -- $(QJSC)

CC := wasicc
CFLAGS := -I$(QUICKJS)

# LDFLAGS := -Wl,--export-all

AR := wasiar

all: jsrunner.wasm
.PHONY: all

jsrunner.wasm: $(OBJS) $(LIBS)
	$(CC) $^ -o $@ $(LDFLAGS)

%.o: %.c
	$(CC) $(CFLAGS) $< -c -o $@

%.js.c: %.js $(QJSC)
	$(run_qjsc) -c -o $@ $<

lodash.js:
	$(MAKE) -C $(LODASH) lodash.bundle.js
	cp $(LODASH)/lodash.bundle.js $@


$(QUICKJS)/%:
	$(MAKE) -C $(QUICKJS) $(@:$(QUICKJS)/%=%) CONFIG_WASI=y AR=$(AR)

.PHONY: clean
clean:
	rm -f jsrunner.wasm *.o *.js.c lodash.js
	$(MAKE) -C $(QUICKJS) clean
