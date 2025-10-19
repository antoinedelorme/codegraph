public class Simple {
    private String name;
    private int value;

    public Simple(String name, int value) {
        this.name = name;
        this.value = value;
    }

    public String getName() {
        return name;
    }

    public void setName(String name) {
        this.name = name;
    }

    public int getValue() {
        return value;
    }

    public void setValue(int value) {
        this.value = value;
    }

    public void printInfo() {
        System.out.println("Name: " + getName() + ", Value: " + getValue());
    }

    public static void main(String[] args) {
        Simple simple = new Simple("Test", 42);
        simple.printInfo();
    }
}