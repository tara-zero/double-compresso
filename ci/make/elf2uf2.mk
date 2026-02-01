%-fw.uf2: %-fw
	elf2uf2-rs convert --family "rp2350-arm-s" "$(<)" "$(@)" >/dev/null
