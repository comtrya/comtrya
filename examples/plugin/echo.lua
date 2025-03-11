local do_this = function(output)
    print(tostring(output.output))
end

return {
    name = "echo",
    version = "0.1.0",
    outcome = "success",
    summary = "Echoed the output",
    plan = function() end,
    exec = do_this,
    -- initializers = {},
    -- finalizers = {},
}
