static var calls = 0;
int initial() {
    calls += 1;
    return 10;
}

struct Mixed {
    var value = 3;
    let ignored = initial();
}
struct Immutable {
    let value : int = initial();
}

RWStructuredBuffer<int> output;
[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    var first = Mixed();
    output[0] = first.value;
    output[1] = calls;
    var second = Mixed(5);
    output[2] = second.value;
    output[3] = calls;
    let third = Immutable(7);
    output[4] = third.value;
    output[5] = calls;
    let fourth = Immutable();
    output[6] = fourth.value;
    output[7] = calls;
}
