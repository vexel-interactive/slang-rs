static var state = 0;
int next() {
    state += 1;
    return state;
}

[ForceInline]
int cached(int input) {
    static var value = next();
    let result = input + value;
    return result;
}

RWStructuredBuffer<int> output;
[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    let unused = next();
    var total = 0;
    for (var index = 0; index < 3; index++) {
        let term = next();
        total += term;
    }
    output[0] = unused;
    output[1] = total;
    output[2] = state;
    output[3] = cached(5);
    output[4] = cached(6);
    output[5] = state;
}
