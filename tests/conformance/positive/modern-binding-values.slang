int choose(int value) { return 1; }
int choose(uint value) { return 2; }
int choose(float value) { return 3; }
int choose(float1 value) { return 4; }
int choose(float2 value) { return 5; }
int choose(int64_t value) { return 6; }

int[2] pair() { return { 3, 4 }; }

struct Item {
    var value = 8;
    let bias : int = 2;

    int read() { return value; }
    [mutating] void write(int input) { value = input; }
}

RWStructuredBuffer<uint> output;
[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    var signedValue = 2;
    let unsignedValue = 3u;
    var floatingValue = 4.5f;
    let one = float1(5);
    var vector = float2(6, 7);
    let wide = 8L;
    let wide64 = 8LL;
    var typed : uint = 9;
    var matrix = float2x2(1, 2, 3, 4);
    let values = pair();
    var item = Item();
    item.write(10);

    const var Count = 3;
    const let Other = Count + 1;
    var sized : int[Other] = { 1, 2, 3, 4 };
    let converted : float2 = int2(2, 3);
    var index = 0;
    let captured = index++;

    output[0] = uint(choose(signedValue));
    output[1] = uint(choose(unsignedValue));
    output[2] = uint(choose(floatingValue));
    output[3] = uint(choose(one));
    output[4] = uint(choose(vector));
    output[5] = uint(choose(wide));
    output[6] = typed;
    output[7] = uint(matrix[1][0]);
    output[8] = uint(values[0] + values[1]);
    output[9] = uint(item.read());
    output[10] = uint(sized[Other - 1]);
    output[11] = uint(converted.y);
    output[12] = uint(captured);
    output[13] = uint(index);
    output[14] = uint(choose(wide64));
}
