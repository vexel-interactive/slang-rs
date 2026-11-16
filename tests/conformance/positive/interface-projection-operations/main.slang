module main;
import lib;
RWStructuredBuffer<uint> output;
[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    GenericValue genericValue;
    ReceiverValue receiver;
    Counter counter = { 2 };
    uint values[2] = { 0x80000000u, 5u };
    int index = 0;
    uint copied;
    output[0] = uint(genericValue.read(1.9));
    output[1] = uint(StaticValue.read(1.9));
    output[2] = uint(ReceiverValue.read(receiver, 1.9));
    output[3] = uint(counter.update(1.9, values[index++], copied));
    output[4] = values[0];
    output[5] = values[1];
    output[6] = uint(index);
    output[7] = copied;
    output[8] = uint(counter.total);
    Inherited inherited;
    output[9] = uint(inherited.read(1.0));
}
