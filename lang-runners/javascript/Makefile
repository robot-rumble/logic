QUICKJS := quickjs/src

LIBS := $(QUICKJS)/libquickjs.a
OBJS := jsrunner.o jsstdlib.o

QJSC := $(QUICKJS)/qjsc.wasm

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

jsstdlib.c: stdlib.js $(QJSC)
	wasmer --dir . -- $(QJSC) -c -o $@ $<

$(QUICKJS)/%:
	$(MAKE) -C $(QUICKJS) $(@:$(QUICKJS)/%=%) AR=$(AR)

.PHONY: clean
clean:
	rm -f jsrunner.wasm *.o jsstdlib.c
	$(MAKE) -C $(QUICKJS) clean
