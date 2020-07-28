QUICKJS := quickjs/src
LODASH := lodash
LOGICCRATE := ../../logic

LIBS := $(QUICKJS)/libquickjs.a
OBJS := jsrunner.o stdlib.js.o lodash.js.o

QJSC := $(QUICKJS)/qjsc.wasm
run_qjsc = wasmer --dir . -- $(QJSC)

YARN = yarn -s
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

stdlib.js: stdlib.ts
	$(YARN) tsc

stdlib.ts: logictypes.d.ts


lodash.js: package.json yarn.lock Makefile
	$(YARN) install
	$(YARN) lodash -d -o $@

$(QUICKJS)/%:
	$(MAKE) -C $(QUICKJS) $(@:$(QUICKJS)/%=%) AR=$(AR)

logictypes.d.ts: $(LOGICCRATE)/Cargo.toml $(LOGICCRATE)/src/*.rs
	echo 'declare module "logictypes" {' > $@
	cargo -q run --manifest-path=$< --bin gen_schema --features=schemars | node ~/json-schema-to-typescript/dist/src/cli.js >> $@
	echo '}' >> $@


.PHONY: clean
clean:
	rm -f jsrunner.wasm *.o *.js.c lodash.js
	$(MAKE) -C $(QUICKJS) clean
