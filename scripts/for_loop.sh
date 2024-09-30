# for i in {1..10}; do
#     echo $i
# done


for i in $(seq 1 2 20); do
    echo $i
done


# for i in {1..10..2}; do
#     echo $i
# done


# for i in $(1,2,3,4,5); do
#     echo $i
# done


for i in $(ls); do
    printf "%s\n" "$i"
done