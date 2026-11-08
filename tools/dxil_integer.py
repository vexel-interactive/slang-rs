"""Bounded interpretation of the integer-only DXIL used by control-flow probes.

This is an independent check of DXC's compiled instructions, not GPU execution
or a general DXIL runtime. Resource reads, floating operations, pointers, and
unsupported instructions fail explicitly. Undefined values may travel through
unselected SSA paths but cannot control a branch or reach an observed store.
"""

import re


UNDEFINED = object()
NAME = r"%[-\w.$]+"
OPERAND = rf"(?:{NAME}|-?\d+|true|false|undef|poison)"
LABEL = r"[-\w.$]+"


def signed(value, width):
    sign = 1 << (width - 1)
    return value - (1 << width) if value & sign else value


def interpret_integer_stores(dump, entry="main", max_steps=100_000):
    """Return the final scalar uint values of the single supported structured UAV."""
    function = re.search(rf"^define void @{re.escape(entry)}\(\)\s*\{{\n(.*?)^\}}", dump, re.M | re.S)
    if function is None:
        raise ValueError("expected a parameterless void DXIL entry")
    blocks = {"0": []}
    current = "0"
    switch_lines = []
    for line in function[1].splitlines():
        if switch_lines:
            switch_lines.append(line.split(";", 1)[0].strip())
            if switch_lines[-1] == "]":
                blocks[current].append(" ".join(switch_lines))
                switch_lines = []
            continue
        label = re.match(rf"\s*; <label>:({LABEL})\b", line) or re.match(rf"\s*({LABEL}):", line)
        if label:
            current = label[1]
            if current in blocks:
                raise ValueError("duplicate DXIL block")
            blocks[current] = []
        else:
            instruction = line.split(";", 1)[0].strip()
            if instruction:
                if instruction.startswith("switch ") and instruction.endswith("["):
                    switch_lines.append(instruction)
                else:
                    blocks[current].append(instruction)
    if switch_lines:
        raise ValueError("unterminated DXIL switch")
    if not blocks["0"] and len(blocks) > 1:
        current = next(label for label in blocks if label != "0")
    else:
        current = "0"
    values = {}
    stores = {}
    resource = None
    predecessor = None
    steps = 0

    def operand(token, width, snapshot=None):
        if not 1 <= width <= 64:
            raise ValueError("unsupported DXIL integer width")
        if token in {"undef", "poison"}:
            return UNDEFINED
        if token.startswith("%"):
            source = values if snapshot is None else snapshot
            if token not in source:
                raise ValueError(f"undefined DXIL value {token}")
            value = source[token]
            if value is not UNDEFINED and not isinstance(value, int):
                raise ValueError("noninteger DXIL operand")
            return value
        if token in {"true", "false"}:
            return int(token == "true")
        return int(token) & ((1 << width) - 1)

    def observed(token, width):
        value = operand(token, width)
        if value is UNDEFINED or not isinstance(value, int):
            raise ValueError("undefined or noninteger observed DXIL value")
        return value

    while True:
        if current not in blocks:
            raise ValueError(f"unknown DXIL block {current}")
        instructions = blocks[current]
        snapshot = values.copy()
        phis = {}
        next_block = None
        for instruction in instructions:
            steps += 1
            if steps > max_steps:
                raise ValueError("DXIL integer instruction budget exceeded")
            if match := re.fullmatch(rf"({NAME}) = phi i(\d+) (.+)", instruction):
                name, width, incoming = match.groups()
                pairs = re.findall(rf"\[\s*({OPERAND}),\s*%({LABEL})\s*\]", incoming)
                chosen = [token for token, label in pairs if label == predecessor]
                if len(chosen) != 1:
                    raise ValueError("ambiguous or missing DXIL phi predecessor")
                phis[name] = operand(chosen[0], int(width), snapshot)
                continue
            # All block parameters take their predecessor values simultaneously.
            if phis:
                values.update(phis)
                phis.clear()
            if match := re.fullmatch(rf"br label %({LABEL})", instruction):
                next_block = match[1]
                break
            if match := re.fullmatch(rf"br i1 ({OPERAND}), label %({LABEL}), label %({LABEL})", instruction):
                next_block = match[2] if observed(match[1], 1) else match[3]
                break
            if instruction == "ret void":
                return stores
            if match := re.fullmatch(rf"switch i(\d+) ({OPERAND}), label %({LABEL}) \[ (.*) \]", instruction):
                width, selector, default, cases = match.groups()
                selected = observed(selector, int(width))
                case_pattern = rf"i{width} (-?\d+), label %({LABEL})"
                pairs = re.findall(case_pattern, cases)
                if re.sub(case_pattern, "", cases).strip():
                    raise ValueError("unsupported DXIL switch cases")
                chosen = [label for number, label in pairs if operand(number, int(width)) == selected]
                if len(chosen) > 1:
                    raise ValueError("duplicate DXIL switch value")
                next_block = chosen[0] if chosen else default
                break
            if match := re.fullmatch(rf"({NAME}) = call %dx.types.Handle @dx.op.createHandle\(i32 57, i8 1, i32 (\d+), i32 (\d+), i1 false\)", instruction):
                name, binding, index = match.groups()
                handle = (int(binding), int(index))
                if resource is not None and resource != handle:
                    raise ValueError("multiple DXIL output resources")
                resource = handle
                values[name] = handle
                continue
            if match := re.fullmatch(rf"call void @dx.op.bufferStore.i32\(i32 69, %dx.types.Handle ({NAME}), i32 ({OPERAND}), i32 0, i32 ({OPERAND}), i32 undef, i32 undef, i32 undef, i8 1\)", instruction):
                handle, index, value = match.groups()
                if resource is None or values.get(handle) != resource:
                    raise ValueError("unknown DXIL output resource")
                stores[str(observed(index, 32))] = observed(value, 32)
                continue
            if match := re.fullmatch(rf"({NAME}) = (zext|sext|trunc) i(\d+) ({OPERAND}) to i(\d+)", instruction):
                name, op, source_width, source, width = match.groups()
                source_width, width = int(source_width), int(width)
                result = operand(source, source_width)
                if result is not UNDEFINED:
                    if op == "sext":
                        result = signed(result, source_width)
                    result &= (1 << width) - 1
                values[name] = result
                continue
            if match := re.fullmatch(rf"({NAME}) = select i1 ({OPERAND}), i(\d+) ({OPERAND}), i\3 ({OPERAND})", instruction):
                name, condition, width, yes, no = match.groups()
                condition = operand(condition, 1)
                values[name] = UNDEFINED if condition is UNDEFINED else operand(yes if condition else no, int(width))
                continue
            if match := re.fullmatch(rf"({NAME}) = icmp (eq|ne|slt|sle|sgt|sge|ult|ule|ugt|uge) i(\d+) ({OPERAND}), ({OPERAND})", instruction):
                name, op, width, a, b = match.groups()
                width = int(width)
                a, b = operand(a, width), operand(b, width)
                if a is UNDEFINED or b is UNDEFINED:
                    values[name] = UNDEFINED
                else:
                    if op.startswith("s"):
                        a, b = signed(a, width), signed(b, width)
                    values[name] = int({"eq": a == b, "ne": a != b, "lt": a < b,
                                        "le": a <= b, "gt": a > b, "ge": a >= b}[op if op in {"eq", "ne"} else op[1:]])
                continue
            if match := re.fullmatch(rf"({NAME}) = (add|sub|mul|and|or|xor|shl|lshr|ashr|udiv|sdiv|urem|srem) ((?:(?:nsw|nuw|exact) )*)i(\d+) ({OPERAND}), ({OPERAND})", instruction):
                name, op, flags, width, a, b = match.groups()
                width = int(width)
                a, b = operand(a, width), operand(b, width)
                if a is UNDEFINED or b is UNDEFINED:
                    values[name] = UNDEFINED
                    continue
                mask = (1 << width) - 1
                if op in {"shl", "lshr", "ashr"} and b >= width or op in {"udiv", "sdiv", "urem", "srem"} and b == 0:
                    values[name] = UNDEFINED
                    continue
                if "exact" in flags:
                    raise ValueError("exact DXIL arithmetic is not implemented")
                if flags and op not in {"add", "sub", "mul"}:
                    raise ValueError("flagged DXIL operation is not implemented")
                if op in {"add", "sub", "mul"}:
                    operation = {"add": lambda a,b: a+b, "sub": lambda a,b: a-b, "mul": lambda a,b: a*b}[op]
                    result = operation(a, b)
                    signed_result = operation(signed(a, width), signed(b, width))
                    if "nuw" in flags and not 0 <= result <= mask or "nsw" in flags and not -(1 << (width - 1)) <= signed_result < (1 << (width - 1)):
                        values[name] = UNDEFINED
                        continue
                elif op in {"udiv", "urem", "sdiv", "srem"}:
                    if op.startswith("s"):
                        a, b = signed(a, width), signed(b, width)
                        if a == -(1 << (width - 1)) and b == -1:
                            values[name] = UNDEFINED
                            continue
                    quotient = (abs(a) // abs(b)) * (-1 if (a < 0) != (b < 0) else 1)
                    result = quotient if op.endswith("div") else a - quotient * b
                elif op == "shl":
                    if flags:
                        raise ValueError("flagged DXIL shifts are not implemented")
                    result = a << b
                elif op == "lshr":
                    result = a >> b
                elif op == "ashr":
                    result = signed(a, width) >> b
                else:
                    result = {"and": lambda a,b: a&b, "or": lambda a,b: a|b, "xor": lambda a,b: a^b}[op](a, b)
                values[name] = result & mask
                continue
            raise ValueError(f"unsupported DXIL integer instruction: {instruction}")
        if next_block is None:
            raise ValueError("DXIL block has no supported terminator")
        predecessor, current = current, next_block
