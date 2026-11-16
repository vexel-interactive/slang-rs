typedef row_major float2x3 Rows;

Rows make() { return Rows(1, 2, 3, 4, 5, 6); }
struct Item {
    var direct = Rows(1, 2, 3, 4, 5, 6);
    var returned = make();
}

RWByteAddressBuffer bytes;
RWStructuredBuffer<uint> output;
[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    var item = Item();
    var copied = item.direct;
    var returned = make();
    var copiedReturn = returned;
    bytes.Store(0, item);
    bytes.Store(48, copied);
    bytes.Store(72, returned);
    bytes.Store(96, copiedReturn);
    output[0] = uint(asfloat(bytes.Load(4)));
    output[1] = uint(asfloat(bytes.Load(28)));
    output[2] = uint(asfloat(bytes.Load(52)));
    output[3] = uint(asfloat(bytes.Load(76)));
    output[4] = uint(asfloat(bytes.Load(100)));
}
