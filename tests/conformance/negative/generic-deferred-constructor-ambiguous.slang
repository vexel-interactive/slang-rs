struct Box<T>
{
    T value;
    __init(T input) { value = input; }
    __init(int input) { value = T(); }
};
int run() { Box<int> value = {3}; return value.value; }
