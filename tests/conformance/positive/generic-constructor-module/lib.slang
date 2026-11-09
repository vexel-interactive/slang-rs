module lib;
public struct Marker
{
    internal int value;
    public __init<T>(T ignored, int input) { value = input; }
};
public int read(Marker input) { return input.value; }
public struct Hidden
{
    internal int value;
    internal __init<T>(T ignored, int input) { value = input; }
};
