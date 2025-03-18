return {
    name = "echo",
    summary = "Echoed the output",
    actions = {
        echo = {
            plan = function() end,
            exec = function(output)
                print(tostring(output.output))
            end,
        },
    },
}
