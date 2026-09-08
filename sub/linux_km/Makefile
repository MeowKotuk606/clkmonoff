obj-m += clk_linux_km.o
clk_linux_km-y := utils/utils.o input.o audio.o displ.o
ldflags-y += -T $(src)/sections.lds
STRIP   := $(shell which llvm-strip || which strip)
OBJCOPY := $(shell which llvm-objcopy || which objcopy)
no-clean-files := compile_commands.json
all:
	$(MAKE) -C /lib/modules/$(shell uname -r)/build M=$(CURDIR) modules
	@ls -lh clk_linux_km.ko
clean:
	@test -f compile_commands.json && mv compile_commands.json .compile_commands.json.bak || true
	$(MAKE) -C /lib/modules/$(shell uname -r)/build M=$(CURDIR) clean
	@test -f .compile_commands.json.bak && mv .compile_commands.json.bak compile_commands.json || true
bear: clean
	bear -- $(MAKE) all
	$(MAKE) clean
	@python3 -c "import json; f='compile_commands.json'; d=[x for x in json.load(open(f)) if not x['file'].endswith('.mod.c') and '/lib/modules/' not in x['file']]; json.dump(d, open(f, 'w'), indent=4)"
.PHONY: all clean bear